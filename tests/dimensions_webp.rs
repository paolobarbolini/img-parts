use std::fs;

use bytes::Bytes;
use img_parts::webp::WebP;

#[test]
fn dimensions_webp_vp8() {
    dimensions_webp_image("P1133897.webp", (717, 512));
}

#[test]
fn dimensions_webp_vp8x() {
    dimensions_webp_image("P1133897_sRGB.webp", (717, 512));
}

#[test]
fn dimensions_webp_out() {
    dimensions_webp_image("P1133897.out.webp", (717, 512));
}

fn dimensions_webp_image(input: &str, dimensions: (u32, u32)) {
    let buf = Bytes::from(fs::read(format!("tests/images/{input}")).expect("read webp"));

    let webp = WebP::from_bytes(buf).unwrap();
    assert_eq!(webp.dimensions(), Some(dimensions));
}
