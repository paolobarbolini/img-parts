use std::fs::{self};

use bytes::Bytes;
use img_parts::jpeg::Jpeg;
use img_parts::ImageEXIF;

#[test]
fn extract_jpeg_noprofile() {
    extract_jpeg_image("P1133897.jpg", None);
}

#[test]
fn extract_jpeg_srgb() {
    extract_jpeg_image("P1133897_sRGB.jpg", Some("P1133897_sRGB.exif"));
}

#[test]
fn extract_jpeg_adobergb() {
    extract_jpeg_image("P1133897_AdobeRGB.jpg", Some("P1133897_AdobeRGB.exif"));
}

fn extract_jpeg_image(input: &str, exif: Option<&str>) {
    let buf = Bytes::from(fs::read(format!("tests/images/{}", input)).expect("read webp"));

    let jpeg = Jpeg::from_bytes(buf).unwrap();
    let exif_meta = jpeg.exif();

    if let Some(exif) = exif {
        let saved = Bytes::from(fs::read(format!("tests/images/{}", exif)).expect("read exif"));
        assert_eq!(exif_meta, Some(saved));
    } else {
        assert!(exif_meta.is_none());
    }
}
