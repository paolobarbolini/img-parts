use std::fs::{self};

use bytes::Bytes;
use img_parts::{jpeg::Jpeg, ImageICC};

#[test]
fn extract_jpeg_noprofile() {
    extract_jpeg_image("P1133897.jpg", None);
}

#[test]
fn extract_jpeg_srgb() {
    extract_jpeg_image("P1133897_sRGB.jpg", Some("P1133897_sRGB.icc"));
}

#[test]
fn extract_jpeg_adobergb() {
    extract_jpeg_image("P1133897_AdobeRGB.jpg", Some("P1133897_AdobeRGB.icc"));
}

#[test]
fn extract_jpeg_plane() {
    extract_jpeg_image("P1133897.plane.jpg", None);
}

fn extract_jpeg_image(input: &str, icc: Option<&str>) {
    let buf = Bytes::from(fs::read(format!("tests/images/{input}")).expect("read jpeg"));

    let jpeg = Jpeg::from_bytes(buf).unwrap();
    let iccp = jpeg.icc_profile();

    if let Some(icc) = icc {
        let iccp = iccp.unwrap();

        let saved = Bytes::from(fs::read(format!("tests/images/{icc}")).expect("read icc"));
        assert_eq!(iccp, saved);
    } else {
        assert!(iccp.is_none());
    }
}
