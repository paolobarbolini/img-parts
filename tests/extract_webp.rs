use std::fs::{self};

use bytes::Bytes;
use img_parts::{webp::WebP, ImageICC};

#[test]
fn extract_webp_noprofile() {
    extract_webp_image("P1133897.webp", None);
}

#[test]
fn extract_webp_srgb() {
    extract_webp_image("P1133897_sRGB.webp", Some("P1133897_sRGB.icc"));
}

#[test]
fn extract_webp_adobergb() {
    extract_webp_image("P1133897_AdobeRGB.webp", Some("P1133897_AdobeRGB.icc"));
}

fn extract_webp_image(input: &str, icc: Option<&str>) {
    let buf = Bytes::from(fs::read(format!("tests/images/{input}")).expect("read webp"));

    let webp = WebP::from_bytes(buf).unwrap();
    let iccp = webp.icc_profile();

    if let Some(icc) = icc {
        let saved = Bytes::from(fs::read(format!("tests/images/{icc}")).expect("read icc"));
        assert_eq!(iccp, Some(saved));
    } else {
        assert!(iccp.is_none());
    }
}
