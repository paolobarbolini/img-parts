use std::fs;

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

    let out = icc_editor::add_icc_to_webp(&mut &file[..], Some(&icc)).unwrap();
    assert_eq!(out, file);
}

fn inject_webp_result(input: &str, output: &str, icc: &str) {
    let file = fs::read(format!("tests/{}", input)).expect("read webp");

    let icc = fs::read(format!("tests/{}", icc)).expect("read icc");

    let out = icc_editor::add_icc_to_webp(&mut &file[..], Some(&icc)).unwrap();
    let out_icc = icc_editor::icc_from_webp(&mut &out[..]).unwrap();
    assert_eq!(out_icc, icc);

    let expected = fs::read(format!("tests/{}", output)).expect("read expected webp");
    assert_eq!(out, expected);
}
