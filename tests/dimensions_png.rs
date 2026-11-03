use std::fs;

use bytes::Bytes;
use img_parts::png::Png;

#[test]
fn dimensions_png() {
    dimensions_png_image("P1133897.png", (717, 512));
}

fn dimensions_png_image(input: &str, dimensions: (u32, u32)) {
    let buf = Bytes::from(fs::read(format!("tests/images/{input}")).expect("read png"));

    let png = Png::from_bytes(buf).unwrap();
    assert_eq!(png.dimensions(), Some(dimensions));
}
