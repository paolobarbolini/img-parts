use std::env::args;
use std::fs::{self, File};
use std::io::{Cursor, Write};
use std::path::Path;
use std::process::exit;

use bytes::Bytes;
use img_parts::{DynImage, ImageEXIF, ImageICC};

fn main() {
    let mut args = args();
    args.next();

    let input_path = match args.next() {
        Some(path) => path,
        None => {
            eprintln!("Please specify the input file path (must be a jpeg, png or webp)");
            exit(1);
        }
    };

    let output_path = match args.next() {
        Some(path) => path,
        None => {
            eprintln!("Please specify the output file path (must have a .jpg or a .png extension)");
            exit(1);
        }
    };

    println!("loading the image...");
    let (img, iccp, exif) = load(input_path.as_ref());

    println!("resizing it...");
    let size = 256;
    let img = img.resize(size, size, image::imageops::FilterType::Lanczos3);

    println!("saving it...");
    save(img, output_path.as_ref(), iccp, exif);
}

/// loads the image from the specified `path`
///
/// Returns the decoded image and the ICCP and EXIF if present
fn load(path: &Path) -> (image::DynamicImage, Option<Bytes>, Option<Bytes>) {
    // read the input file
    let buf = fs::read(path).expect("read input file");

    // load the image
    let img = image::load_from_memory(&buf).expect("image decoded");

    // extract ICC and EXIF metadata
    let (iccp, exif) = DynImage::from_bytes(buf.into())
        .expect("image loaded")
        .map_or((None, None), |dimg| (dimg.icc_profile(), dimg.exif()));

    (img, iccp, exif)
}

/// saves the image to the specified `path`
///
/// If `iccp` *and* `exif` are None, the image is written directly to `path`, else
/// it is buffered in memory, modified by img-parts and then written to `path`.
fn save(img: image::DynamicImage, path: &Path, iccp: Option<Bytes>, exif: Option<Bytes>) {
    let out_format = image::ImageFormat::from_path(&path).expect("detected output format");
    let mut out_file = File::create(path).expect("create output file");

    if iccp.is_some() || exif.is_some() {
        let mut out = Vec::new();
        img.write_to(&mut Cursor::new(&mut out), out_format)
            .expect("image encoded");
        let out = Bytes::from(out);

        match DynImage::from_bytes(out.clone()).expect("image loaded") {
            Some(mut dimg) => {
                dimg.set_icc_profile(iccp);
                dimg.set_exif(exif);
                dimg.encoder()
                    .write_to(out_file)
                    .expect("output file written");
            }
            None => out_file.write_all(&out).expect("output file written"),
        };
    } else {
        img.write_to(&mut out_file, out_format)
            .expect("image encoded without ICCP or EXIF");
    }
}
