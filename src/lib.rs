//! A super simple no std psf2 parser for rust.
//!
//! The psfu format is what's used in the linux tty.
//! You can find the built in psf2 fonts in /usr/share/kbd/consolefonts.
//!
//! This doesn't support the original psf.

#![no_std]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::indexing_slicing,
    clippy::as_conversions,
    clippy::cast_lossless
)]

use core::panic;

mod tests;

type HashMap = heapless::IndexMap<[u8; 4], usize, hash32::BuildHasherDefault<ahash::AHasher>, 1024>;

/// Magic bytes that identify psf2.
const MAGIC: [u8; 4] = [0x72, 0xb5, 0x4a, 0x86];

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
pub struct Font<'a> {
    /// The font header for this font.
    pub header: Header,

    /// The data NOT including the header.
    data: &'a [u8],

    /// The parsed unicode table.
    unicode: Option<HashMap>,
}

impl<'a> Font<'a> {
    /// Converts the unicode table in a font to a hashmap.
    ///
    /// # Arguments
    ///
    /// * `table` - A byte slice of the actual unicode table.
    fn parse_unicode_table(table: &[u8]) -> HashMap {
        let mut result: HashMap = HashMap::new();

        for (i, entry) in table.split(|x| x == &0xff).enumerate() {
            let mut iter = entry.iter().enumerate();
            while let Some((j, byte)) = iter.next() {
                let utf8_len = match byte >> 4usize {
                    0xc | 0xd => 2,
                    0xe => 3,
                    0xf => 4,
                    _ => 1,
                };

                let mut key = [0; 4];

                key[..utf8_len].copy_from_slice(&entry[j..j + utf8_len]);
                result.insert(key, i).unwrap();

                for _ in 0..utf8_len - 1 {
                    if iter.next().is_none() {
                        break;
                    }
                }
            }
        }

        result
    }

    /// Gets the glyph index of a character by using the fonts own unicode table.
    /// This index is where the glyph in the font itself.
    ///
    /// # Arguments
    ///
    /// * `char` - The Unicode Scalar Value of the character you want the index of. (Just cast a char to u32)
    ///
    /// # Panics
    ///
    /// * If the unicode table flag is set to true, but the table hasn't yet been defined.
    fn glyph_index(&self, char: u32) -> Option<usize> {
        // Should work for basic ASCII.
        if !self.header.flags.unicode || char < 128 {
            return Some(char as usize);
        }

        let mut utf8 = [0; 4];
        char::from_u32(char).unwrap().encode_utf8(&mut utf8);

        self.unicode
            .as_ref()
            .expect("unicode table doesn't exist, but header states otherwise")
            .get(&utf8)
            .copied()
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

        let char = self.glyph_index(char).map_or('?' as usize, |value| value) as u32;

        let from = self.header.glyph_size * (char);
        let to = self.header.glyph_size * (char + 1);

        let data = &self.data[from as usize..to as usize];
        let bytes_in_row = ((self.header.glyph_width as usize + 7) & !7) / 8;

        for (i, row) in data.chunks(bytes_in_row).enumerate() {
            'row: for (j, byte) in row.iter().enumerate() {
                for k in 0..8 {
                    let x = (j as u8 * 8) + k;

                    if x as u32 > self.header.glyph_width {
                        break 'row;
                    }

                    // Bit is a u8 that is always either a 0 or a 1.
                    // "But why not use a boolean?" I hear you ask.
                    // Every variable in rust is always at least one byte in size,
                    // So it doesn't do much for saving memory.
                    let bit = (byte >> (7 - k)) & 1;

                    action(bit, x, i as u8);
                }
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
    pub fn load(raw: &'a [u8]) -> Self {
        let header_size = as_u32_le(&raw[0x8..0xc]);
        let header = Header {
            magic: [raw[0x0], raw[0x1], raw[0x2], raw[0x3]],
            version: as_u32_le(&raw[0x4..0x8]),
            size: header_size,
            flags: Flags::parse(&raw[0xc..0x10]),
            length: as_u32_le(&raw[0x10..0x14]),
            glyph_size: as_u32_le(&raw[0x14..0x18]),
            glyph_height: as_u32_le(&raw[0x18..0x1c]),
            glyph_width: as_u32_le(&raw[0x1c..0x20]),
        };
        let data = &raw[header_size as usize..];

        let font = Self {
            header,
            data,
            unicode: Some(Self::parse_unicode_table(
                &raw[(header.glyph_size * header.length) as usize..],
            )),
        };

        assert!(
            font.header.magic == MAGIC,
            "header magic does not match, is this a psf2 font?"
        );

        font
    }
}

/// Converts an array of u8's into one u32.
const fn as_u32_le(array: &[u8]) -> u32 {
    assert!(
        array.len() > 3,
        "`array` needs to have four elements or more"
    );

    (array[0] as u32)
        + ((array[1] as u32) << 8u32)
        + ((array[2] as u32) << 16u32)
        + ((array[3] as u32) << 24u32)
}
