//! A super simple no std psf2 parser for rust.
//!
//! The psfu format is what's used in the linux tty.
//! You can find the built in psf2 fonts in /usr/share/kbd/consolefonts.
//!
//! This doesn't support the original psf yet, and currently doesn't support glyphs that aren't 8px wide.

#![no_std]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo
)]

#![allow(
    clippy::cast_possible_truncation,
    clippy::indexing_slicing,
    clippy::as_conversions,
    clippy::cast_lossless
)]

/// The typo is intentional now :^)
const FILE_ZIZE: usize = 4969;

/// The font header.
#[derive(Clone, Copy, Debug)]
pub struct Header {
    pub magic: [u8; 4],
    pub version: u32,
    pub size: u32,
    pub flags: u32,
    pub length: u32,
    pub glyph_size: u32,
    pub glyph_height: u32,
    pub glyph_width: u32,
}

/// The structure for the font.
///
/// # Example
///
/// ```rust
/// let font = Font::load(fs::read("./font.psfu").unwrap().as_slice());
///
/// font.get_char('A', |bit, x, y| {
///    // Stuff
/// });
/// ```
#[derive(Debug)]
pub struct Font {
    pub header: Header,
    char_data: [u8; FILE_ZIZE - 32],
}

impl Font {
    /// # Arguments
    ///
    /// * `char` - Pretty self explanitory. A character, that must be ASCII.
    /// * `action` - A closure that takes in 3 values, the bit (always 0 or 1), the x, and the y.
    pub fn get_char(&self, char: char, mut action: impl FnMut(u8, u8, u8)) {
        let char = char as u32;

        let from = self.header.glyph_size * (char);
        let to = self.header.glyph_size * (char + 1);

        for (i, byte) in self.char_data[from as usize..to as usize]
            .iter()
            .enumerate()
        {
            if byte == &0 {
                continue;
            }

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
    /// * If the file size doesn't correspond with the defined const.
    /// 
    #[inline]
    #[must_use]
    pub fn load(raw: &[u8]) -> Self {
        let size = as_u32_le(&raw[0x8..0xc]);

        let font = Self {
            header: Header {
                magic: [raw[0x0], raw[0x1], raw[0x2], raw[0x3]],
                version: as_u32_le(&raw[0x4..0x8]),
                size,
                flags: as_u32_le(&raw[0xc..0x10]),
                length: as_u32_le(&raw[0x10..0x14]),
                glyph_size: as_u32_le(&raw[0x14..0x18]),
                glyph_height: as_u32_le(&raw[0x18..0x1c]),
                glyph_width: as_u32_le(&raw[0x1c..0x20]),
            },
            char_data: raw[size as usize..].try_into().unwrap(),
        };

        #[allow(clippy::manual_assert)]
        if font.header.magic != [0x72, 0xb5, 0x4a, 0x86] {
            panic!("header magic does not match, is this a psf2 font?");
        }

        font
    }
}

/// Converts an array of u8's into one u32.
const fn as_u32_le(array: &[u8]) -> u32 {
    (array[0] as u32)
        + ((array[1] as u32) << 8)
        + ((array[2] as u32) << 16)
        + ((array[3] as u32) << 24)
}
