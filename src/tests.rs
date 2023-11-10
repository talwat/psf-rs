#![cfg(test)]

use crate::Font;

#[test]
fn glyph_index() {
    let font = Font::load(include_bytes!("../test.psfu"));

    assert_eq!(font.glyph_index('A' as u32), Some(0x41));
    assert_eq!(font.glyph_index('~' as u32), Some(0x7e));
    assert_eq!(font.glyph_index('µ' as u32), Some(0xe6));
    assert_eq!(font.glyph_index('¶' as u32), Some(0x14));
    assert_eq!(font.glyph_index('²' as u32), Some(0xfd));
    assert_eq!(font.glyph_index('Σ' as u32), Some(0xe4));
    assert_eq!(font.glyph_index('╝' as u32), Some(0xbc));
    assert_eq!(font.glyph_index('µ' as u32), Some(0xe6));
    assert_eq!(font.glyph_index('μ' as u32), Some(0xe6));

    const OMEGA_1: char = 'Ω';
    const OMEGA_2: char = 'Ω';

    assert_eq!(font.glyph_index(OMEGA_1 as u32), Some(0xea));
    assert_eq!(font.glyph_index(OMEGA_2 as u32), Some(0xea));

    assert_ne!(OMEGA_1, OMEGA_2);
}
