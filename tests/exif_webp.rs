use std::fs::{self, File};
use std::io::BufReader;

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
fn extract_webp_adobergb() {
    extract_webp_image("P1133897_AdobeRGB.webp", Some("P1133897_AdobeRGB.exif"));
}

fn extract_webp_image(input: &str, exif: Option<&str>) {
    let file = File::open(format!("tests/images/{}", input)).expect("open webp");
    let mut reader = BufReader::new(file);

    let webp = WebP::read(&mut reader).unwrap();
    let exif_meta = webp.exif();

    if let Some(exif) = exif {
        let saved = fs::read(format!("tests/images/{}", exif)).expect("read exif");
        assert_eq!(exif_meta, Some(saved));
    } else {
        assert!(exif_meta.is_none());
    }
}
