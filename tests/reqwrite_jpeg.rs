use std::fs;

use bytes::Bytes;
use img_parts::jpeg::Jpeg;

#[test]
fn reqwrite_jpeg_noprofile() {
    extract_jpeg_image("P1133897.jpg");
}

#[test]
fn reqwrite_jpeg_srgb() {
    extract_jpeg_image("P1133897_sRGB.jpg");
}

#[test]
fn reqwrite_jpeg_adobergb() {
    extract_jpeg_image("P1133897_AdobeRGB.jpg");
}

#[test]
fn reqwrite_jpeg_plane() {
    extract_jpeg_image("P1133897.plane.jpg");
}

fn extract_jpeg_image(input: &str) {
    let file = Bytes::from(fs::read(format!("tests/images/{}", input)).expect("read jpeg"));

    let jpeg = Jpeg::from_bytes(file.clone()).unwrap();

    let mut bytes = Vec::new();
    jpeg.write_to(&mut bytes).expect("write jpeg");
    assert_eq!(file, bytes);
    assert_eq!(file.len(), jpeg.len());
}
