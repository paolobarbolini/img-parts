use std::fs;

use icc_editor::{RiffChunk, WebP};

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
    let file = fs::read(format!("tests/{}", input)).expect("read webp");
    let icc = fs::read(format!("tests/{}", icc)).expect("read icc");

    let mut webp = WebP::read(&mut &file[..]).unwrap();
    webp.remove_chunks_by_id(*b"ICCP");

    let chunk = RiffChunk::new(*b"ICCP", icc);
    webp.chunks_mut().insert(0, chunk);

    let mut out = Vec::new();
    webp.write_to(&mut out).expect("write webp");

    assert_eq!(out, file);
}

fn inject_webp_result(input: &str, output: &str, icc: &str) {
    let file = fs::read(format!("tests/{}", input)).expect("read webp");
    let icc = fs::read(format!("tests/{}", icc)).expect("read icc");

    let mut webp = WebP::read(&mut &file[..]).expect("parse webp");
    webp.remove_chunks_by_id(*b"ICCP");

    let chunk = RiffChunk::new(*b"ICCP", icc);
    webp.chunks_mut().insert(0, chunk);

    let mut out = Vec::new();
    webp.write_to(&mut out).expect("write webp");

    let expected = fs::read(format!("tests/{}", output)).expect("read expected webp");
    assert_eq!(out, expected);
}
