use std::fs;

use bytes::Bytes;
use img_parts::jpeg::Jpeg;

#[test]
fn dimensions_jpeg_baseline() {
    dimensions_jpeg_image("P1133897.jpg", (717, 512));
}

#[test]
fn dimensions_jpeg_progressive() {
    dimensions_jpeg_image("P1133897.plane.jpg", (717, 512));
}

#[test]
fn dimensions_jpeg_exif() {
    dimensions_jpeg_image("P1133897_sRGB.jpg", (717, 512));
}

fn dimensions_jpeg_image(input: &str, dimensions: (u16, u16)) {
    let buf = Bytes::from(fs::read(format!("tests/images/{input}")).expect("read jpeg"));

    let jpeg = Jpeg::from_bytes(buf).unwrap();
    assert_eq!(jpeg.dimensions(), Some(dimensions));
}
