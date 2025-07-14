use std::fs::{self};

use bytes::Bytes;
use img_parts::webp::WebP;
use img_parts::ImageEXIF;

#[test]
fn extract_webp_noprofile() {
    extract_webp_image("P1133897.webp", None);
}

#[test]
fn extract_webp_srgb() {
    extract_webp_image("P1133897_sRGB.webp", Some("P1133897_sRGB.exif"));
}

#[test]
fn extract_webp_nomagic_exif() {
    extract_webp_image(
        "P1133897_AdobeRGB_exif.webp",
        Some("P1133897_AdobeRGB.exif"),
    );
}

#[test]
fn extract_webp_adobergb() {
    extract_webp_image("P1133897_AdobeRGB.webp", Some("P1133897_AdobeRGB.exif"));
}

fn extract_webp_image(input: &str, exif: Option<&str>) {
    let buf = Bytes::from(fs::read(format!("tests/images/{input}")).expect("read webp"));

    let webp = WebP::from_bytes(buf).unwrap();
    let exif_meta = webp.exif();

    if let Some(exif) = exif {
        let saved = Bytes::from(fs::read(format!("tests/images/{exif}")).expect("read exif"));
        assert_eq!(exif_meta, Some(saved));
    } else {
        assert!(exif_meta.is_none());
    }
}
