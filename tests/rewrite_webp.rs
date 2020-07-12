use std::fs;

use bytes::Bytes;
use img_parts::webp::WebP;

#[test]
fn reqwrite_webp_noprofile() {
    extract_webp_image("P1133897.webp");
}

#[test]
fn reqwrite_webp_srgb() {
    extract_webp_image("P1133897_sRGB.webp");
}

#[test]
fn reqwrite_webp_adobergb() {
    extract_webp_image("P1133897_AdobeRGB.webp");
}

#[test]
fn reqwrite_webp_out() {
    extract_webp_image("P1133897.out.webp");
}

fn extract_webp_image(input: &str) {
    let file = Bytes::from(fs::read(format!("tests/images/{}", input)).expect("read webp"));

    let webp = WebP::from_bytes(file.clone()).unwrap();
    let webp_len = webp.len();

    let mut bytes = Vec::new();
    webp.write_to(&mut bytes).expect("write webp");
    assert_eq!(file, bytes);
    assert_eq!(file.len(), webp_len as usize);
}
