use std::fs;

use bytes::Bytes;
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
    let file = Bytes::from(fs::read(format!("tests/images/{input}")).expect("read jpeg"));
    let icc = Bytes::from(fs::read(format!("tests/images/{icc}")).expect("read icc"));

    let mut jpeg = Jpeg::from_bytes(file.clone()).unwrap();
    jpeg.set_icc_profile(Some(icc));

    let out = jpeg.encoder().bytes();
    assert_eq!(out, file);
}

fn inject_jpeg_result(input: &str, output: &str, icc: &str) {
    let file = Bytes::from(fs::read(format!("tests/images/{input}")).expect("read jpeg"));
    let icc = Bytes::from(fs::read(format!("tests/images/{icc}")).expect("read icc"));

    let mut jpeg = Jpeg::from_bytes(file).expect("parse jpeg");
    jpeg.set_icc_profile(Some(icc));

    let out = jpeg.encoder().bytes();

    let expected =
        Bytes::from(fs::read(format!("tests/images/{output}")).expect("read expected jpeg"));
    assert_eq!(out, expected);
}
