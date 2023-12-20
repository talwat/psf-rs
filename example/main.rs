use psf_rs::Font;

fn main() {
    let font = Font::load(include_bytes!("../test.psfu"));

    font.display_glyph('&', |bit, x, y| {
        if x == 0 && y != 0 {
            println!();
        }

        print!("{}", if bit == 1 { "@" } else { " " });
    });

    println!()
}