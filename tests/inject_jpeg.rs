use std::fs;

use img_parts::jpeg::Jpeg;
use img_parts::ImageICC;

#[test]
fn inject_jpeg_noop1() {
    inject_jpeg_noop("P1133897_sRGB.jpg", "P1133897_sRGB.icc");
}

#[test]
fn inject_jpeg_noop2() {
    inject_jpeg_noop("P1133897_AdobeRGB.jpg", "P1133897_AdobeRGB.icc");
}

#[test]
fn inject_jpeg_result1() {
    inject_jpeg_result("P1133897.jpg", "P1133897.out.jpg", "P1133897_AdobeRGB.icc");
}

fn inject_jpeg_noop(input: &str, icc: &str) {
    let file = fs::read(format!("tests/images/{}", input)).expect("read jpeg");
    let icc = fs::read(format!("tests/images/{}", icc)).expect("read icc");

    let mut jpeg = Jpeg::read(&mut &file[..]).unwrap();
    jpeg.set_icc_profile(Some(icc));

    let mut out = Vec::new();
    jpeg.write_to(&mut out).expect("write jpeg");
    assert_eq!(out, file);
}

fn inject_jpeg_result(input: &str, output: &str, icc: &str) {
    let file = fs::read(format!("tests/images/{}", input)).expect("read jpeg");
    let icc = fs::read(format!("tests/images/{}", icc)).expect("read icc");

    let mut jpeg = Jpeg::read(&mut &file[..]).expect("parse jpeg");
    jpeg.set_icc_profile(Some(icc));

    let mut out = Vec::new();
    jpeg.write_to(&mut out).expect("write jpeg");

    let expected = fs::read(format!("tests/images/{}", output)).expect("read expected jpeg");
    assert_eq!(out, expected);
}
