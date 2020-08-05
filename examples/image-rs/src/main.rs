use std::env::args;
use std::fs::{self, File};
use std::path::Path;
use std::process::exit;

use bytes::buf::BufMutExt;
use bytes::{Bytes, BytesMut};
use img_parts::jpeg::Jpeg;
use img_parts::png::Png;
use img_parts::webp::WebP;
use img_parts::{ImageEXIF, ImageICC};

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
    let format = image::guess_format(&buf).expect("format is supported");
    let img = image::load_from_memory_with_format(&buf, format).expect("image loaded");

    // extract ICC and EXIF metadata
    let (iccp, exif) = match format {
        image::ImageFormat::Jpeg => {
            let jpeg =
                Jpeg::from_bytes(buf.into()).expect("img-parts successfully loaded the jpeg");
            (jpeg.icc_profile(), jpeg.exif())
        }
        image::ImageFormat::Png => {
            let png = Png::from_bytes(buf.into()).expect("img-parts successfully loaded the png");
            (png.icc_profile(), png.exif())
        }
        image::ImageFormat::WebP => {
            let webp =
                WebP::from_bytes(buf.into()).expect("img-parts successfully loaded the webp");
            (webp.icc_profile(), webp.exif())
        }
        _ => {
            // unsupported img-parts format
            (None, None)
        }
    };

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
        match out_format {
            image::ImageFormat::Jpeg => {
                let mut out = BytesMut::new().writer();
                img.write_to(&mut out, out_format).expect("image encoded");
                let out = out.into_inner().freeze();

                let mut jpeg = Jpeg::from_bytes(out).expect("img-parts loaded the jpg");
                jpeg.set_icc_profile(iccp);
                jpeg.set_exif(exif);
                jpeg.encoder()
                    .write_to(out_file)
                    .expect("output file written");
                return;
            }
            image::ImageFormat::Png => {
                let mut out = BytesMut::new().writer();
                img.write_to(&mut out, out_format).expect("image encoded");
                let out = out.into_inner().freeze();

                let mut png = Png::from_bytes(out).expect("img-parts loaded the png");
                png.set_icc_profile(iccp);
                png.set_exif(exif);
                png.encoder()
                    .write_to(out_file)
                    .expect("output file written");
                return;
            }
            _ => {}
        };
    }

    img.write_to(&mut out_file, out_format)
        .expect("image encoded without ICCP or EXIF");
}
