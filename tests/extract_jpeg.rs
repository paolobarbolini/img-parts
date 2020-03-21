use std::fs::{self, File};
use std::io::BufReader;

use icc_editor::jpeg::Jpeg;

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

fn extract_jpeg_image(input: &str, icc: Option<&str>) {
    let file = File::open(format!("tests/{}", input)).expect("open jpeg");
    let mut reader = BufReader::new(file);

    let jpeg = Jpeg::read(&mut reader).unwrap();
    let iccp = jpeg.icc_profile();

    if let Some(icc) = icc {
        let iccp = iccp.unwrap();

        let saved = fs::read(format!("tests/{}", icc)).expect("read icc");
        assert_eq!(iccp, saved.as_slice());
    } else {
        assert!(iccp.is_none());
    }
}
