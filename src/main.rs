use psf_rs::Font;

fn main() {
    let raw = include_bytes!("../test.psfu");
    let font = Font::load(raw);

    font.display_glyph('μ', |bit, x, y| {
        if x == 0 {
            println!();
        }

        print!("{}", if bit == 1 { "@" } else { " " });
    });

    println!()
}
