use std::fs::{self, File};
use std::io::BufReader;

use icc_editor::webp::{WebP, CHUNK_ICCP};

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

    let webp = WebP::read(&mut reader).unwrap();
    let iccp = webp.chunk_by_id(CHUNK_ICCP);

    if let Some(icc) = icc {
        let iccp = iccp.unwrap();

        let saved = fs::read(format!("tests/{}", icc)).expect("read icc");
        assert_eq!(iccp.contents(), saved.as_slice());
    } else {
        assert!(iccp.is_none());
    }
}
