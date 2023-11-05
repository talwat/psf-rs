//! A super simple no std psf2 parser for rust.
//!
//! The psfu format is what's used in the linux tty.
//! You can find the built in psf2 fonts in /usr/share/kbd/consolefonts.
//!
//! This doesn't support the original psf yet, and currently doesn't support glyphs that aren't 8px wide.

#![no_std]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::indexing_slicing,
    clippy::as_conversions,
    clippy::cast_lossless
)]

use core::panic;

/// Magic bytes that identify psf2.
const MAGIC: [u8; 4] = [0x72, 0xb5, 0x4a, 0x86];

/// The typo is intentional now :^)
///
/// The maximum size a font can be in bytes.
/// This amount of bytes is allocated on the stack each font!
const FILE_ZIZE: usize = 0x4000;

/// Font flags.
///
/// Currently, there is only one flag that specifies
/// whether there is a unicode table or not.
#[derive(Clone, Copy, Debug)]
pub struct Flags {
    /// Whether a unicode table is present or not.
    pub unicode: bool,
}

impl Flags {
    /// Parses the flags from four bytes.
    const fn parse(raw: &[u8]) -> Self {
        Self {
            unicode: raw[0] == 1,
        }
    }
}

/// The font header.
#[derive(Clone, Copy, Debug)]
pub struct Header {
    /// Magic that is consistent among all psfu files.
    pub magic: [u8; 4],

    /// The version of psfu used. Currently it's always 0.
    pub version: u32,

    /// The size of the header in bytes. Pretty much always 32.
    pub size: u32,

    /// Flags that specify a few things about the font. Currently there's only one.
    pub flags: Flags,

    /// The number of glyphs.
    pub length: u32,

    /// The size in bytes of each glyph.
    pub glyph_size: u32,

    /// The height of each glyph. In this library it always equals `glyph_size`.
    pub glyph_height: u32,

    /// The width of the glyphs.
    pub glyph_width: u32,
}

/// The structure for the font.
///
/// # Example
///
/// ```rust
/// use psf_rs::Font;
///
/// let font = Font::load(include_bytes!("../test.psfu"));
///
/// font.display_glyph('A', |bit, x, y| {
///    // Stuff
/// });
/// ```
#[derive(Debug)]
pub struct Font {
    /// The font header for this font.
    pub header: Header,

    /// The data NOT including the header.
    data: [u8; FILE_ZIZE - 32],

    /// The size of the original font.
    /// Useful, since the font will be put into an array 0x4000 (16384) bytes long regardless.
    size: usize,
}

impl Font {
    /// Gets the glyph index of a character by using the fonts own unicode table.
    /// This index is where the glyph in the font itself.
    ///
    /// # Arguments
    ///
    /// * `char` - The Unicode Scalar Value of the character you want the index of. (Just cast to u32)
    ///
    /// # Panics
    ///
    /// * If the character can't be described with 2 bytes or less in UTF-8.
    fn glyph_index(&self, char: u32) -> u32 {
        // TODO: Make this function faster, since this can take ages.

        // Should work for basic ASCII.
        if !self.header.flags.unicode || char < 128 {
            return char;
        }

        let table = &self.data[(self.header.glyph_size * self.header.length) as usize..self.size];
        let mut index = 63; // '?' Is a reasonable default.

        for (i, entry) in table.split(|x| x == &0xff).enumerate() {
            // Rust doesn't expose `encode_utf8_raw` without a feature.
            // And in interest of keeping this crate on the stable toolchain,
            // we have to do this less than ideal hack to get it to work.
            //
            // We need the `encode_utf8` function to convert the normal
            // unicode codepoint to valid UTF-8.
            //
            // Only allocating 2 bytes because psf2 fonts can only have fonts that
            // can be described with 2 bytes.
            let mut utf8 = [0; 4];
            char::from_u32(char).unwrap().encode_utf8(&mut utf8);

            let mut len = 0;
            for byte in utf8 {
                if byte != 0 {
                    len += 1;
                }
            }

            for j in 0..entry.len() {
                if j + len > entry.len() {
                    break;
                }

                let compare = &entry[j..(j + len)];

                // Using a slice of `utf8` is a bit of a hack.
                // Because we don't need to worry about empty space since a match will always be the exact same size.
                if compare == &utf8[..compare.len()] {
                    index = i;

                    break;
                }
            }
        }

        index as u32
    }

    /// Displays a glyph.
    /// This will NOT trim the glyph, so you will still get the vertical padding.
    ///
    /// # Arguments
    ///
    /// * `char` - Pretty self explanitory. A character or integer, that must represent a glyph on the ASCII table.
    /// * `action` - A closure that takes in 3 values, the bit (always 0 or 1), the x, and the y.
    ///
    /// # Panics
    ///
    /// * If the character can't be properly converted into a u32.
    /// * If the character can't be described with 2 bytes or less in UTF-8.
    pub fn display_glyph<T: TryInto<u32>>(&self, char: T, mut action: impl FnMut(u8, u8, u8)) {
        let Ok(char) = TryInto::<u32>::try_into(char) else {
            panic!("invalid character index")
        };

        let char = self.glyph_index(char);
        let from = self.header.glyph_size * (char);
        let to = self.header.glyph_size * (char + 1);

        for (i, byte) in self.data[from as usize..to as usize].iter().enumerate() {
            for j in 0..8 {
                // Bit is a u8 that is always either a 0 or a 1.
                // "But why not use a boolean?" I hear you ask.
                // Every variable in rust is always at least one byte in size,
                // So it doesn't do much for saving memory.
                let bit = (byte >> (7 - j)) & 1;

                action(bit, j, i as u8);
            }
        }
    }

    /// Loads a font.
    ///
    /// # Arguments
    ///
    /// * `raw` - The raw bytes for the font file itself.
    ///
    /// # Panics
    ///
    /// * If the file header is incomplete/corrupted in pretty much any way.
    /// * If the magic doesn't match.
    /// * If the file size doesn't is bigger than 0x4000 (16384) bytes.
    #[must_use]
    pub fn load(raw: &[u8]) -> Self {
        let size = as_u32_le(&raw[0x8..0xc]);

        // Allocate a slice filled with 0's.
        // This is a temporary solution that will generally work okay for ASCII fonts.
        // TODO: Figure out a solution that isn't this hack,
        // TODO: While still using the stack only.
        let mut data = [0; FILE_ZIZE];

        // Then copy the font into said slice.
        // There will be a lot of empty padding.
        data[..raw.len()].copy_from_slice(raw);

        let font = Self {
            header: Header {
                magic: [raw[0x0], raw[0x1], raw[0x2], raw[0x3]],
                version: as_u32_le(&raw[0x4..0x8]),
                size,
                flags: Flags::parse(&raw[0xc..0x10]),
                length: as_u32_le(&raw[0x10..0x14]),
                glyph_size: as_u32_le(&raw[0x14..0x18]),
                glyph_height: as_u32_le(&raw[0x18..0x1c]),
                glyph_width: as_u32_le(&raw[0x1c..0x20]),
            },
            data: data[size as usize..].try_into().unwrap(),
            size: raw.len(),
        };

        #[allow(clippy::manual_assert)]
        if font.header.magic != MAGIC {
            panic!("header magic does not match, is this a psf2 font?");
        }

        font
    }
}

/// Converts an array of u8's into one u32.
const fn as_u32_le(array: &[u8]) -> u32 {
    (array[0] as u32)
        + ((array[1] as u32) << 8u32)
        + ((array[2] as u32) << 16u32)
        + ((array[3] as u32) << 24u32)
}
