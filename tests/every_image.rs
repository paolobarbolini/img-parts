use std::fs;

use img_parts::{Bytes, DynImage};

#[test]
fn read_write_every_image() {
    let dir = fs::read_dir("tests/images/").expect("read_dir");

    let mut tested = 0;
    for file in dir {
        let file = file.expect("file");
        let file = file.path();

        match file.extension().unwrap().to_str().unwrap() {
            "icc" | "exif" => continue,
            _ => (),
        };

        let buf = fs::read(file).expect("read");
        let buf = Bytes::from(buf);

        let img = DynImage::from_bytes(buf.clone())
            .expect("from_bytes")
            .expect("supported");
        assert_eq!(img.len(), buf.len());
        let out = img.encoder().bytes();
        assert_eq!(out.len(), buf.len());
        assert_eq!(out, buf);

        tested += 1;
    }

    assert_eq!(tested, 13);
}
