# psf-rs

A super simple rust no std psf2 parser that operates entirely on the stack.

Install with `cargo add psf-rs`

## Note

Due to a temporary solution, all fonts must be under 8192 bytes in size.
This is to ensure that everything is allocated on the stack at compile time.
