use std::fs;

use bytes::Bytes;
use img_parts::png::Png;

#[test]
fn rewrite_png_noprofile() {
    extract_png_image("P1133897.png");
}

fn extract_png_image(input: &str) {
    let file = Bytes::from(fs::read(format!("tests/images/{}", input)).expect("read png"));

    let png = Png::from_bytes(file.clone()).unwrap();
    let png_len = png.len();

    let out = png.encoder().bytes();
    assert_eq!(file, out);
    assert_eq!(file.len(), png_len as usize);
}
