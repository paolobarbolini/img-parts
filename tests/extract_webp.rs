use std::fs::{self, File};
use std::io::BufReader;

use icc_editor::Error;

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
    let file = File::open(format!("tests/{}", input)).expect("open webp");
    let mut reader = BufReader::new(file);

    let output = icc_editor::icc_from_webp(&mut reader);

    if let Some(icc) = icc {
        let output = output.unwrap();

        let saved = fs::read(format!("tests/{}", icc)).expect("read icc");
        assert_eq!(output, saved);
    } else {
        match output {
            Err(Error::NoVP8X) => {}
            _ => panic!("expected: 'Error::NoVP8X' got: {:?}", output),
        }
    }
}
