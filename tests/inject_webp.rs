use std::fs;

use img_parts::webp::WebP;
use img_parts::ImageICC;

#[test]
fn inject_webp_noop1() {
    inject_webp_noop("P1133897_sRGB.webp", "P1133897_sRGB.icc");
}

#[test]
fn inject_webp_noop2() {
    inject_webp_noop("P1133897_AdobeRGB.webp", "P1133897_AdobeRGB.icc");
}

#[test]
fn inject_webp_result1() {
    inject_webp_result(
        "P1133897.webp",
        "P1133897.out.webp",
        "P1133897_AdobeRGB.icc",
    );
}

fn inject_webp_noop(input: &str, icc: &str) {
    let file = fs::read(format!("tests/images/{}", input)).expect("read webp");
    let icc = fs::read(format!("tests/images/{}", icc)).expect("read icc");

    let mut webp = WebP::read(&mut &file[..]).unwrap();
    webp.set_icc_profile(Some(icc));

    let mut out = Vec::new();
    webp.write_to(&mut out).expect("write webp");

    assert_eq!(out, file);
}

fn inject_webp_result(input: &str, output: &str, icc: &str) {
    let file = fs::read(format!("tests/images/{}", input)).expect("read webp");
    let icc = fs::read(format!("tests/images/{}", icc)).expect("read icc");

    let mut webp = WebP::read(&mut &file[..]).expect("parse webp");
    webp.set_icc_profile(Some(icc));

    let mut out = Vec::new();
    webp.write_to(&mut out).expect("write webp");

    let expected = fs::read(format!("tests/images/{}", output)).expect("read expected webp");
    assert_eq!(out, expected);
}
