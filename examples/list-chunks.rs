use std::env::args;
use std::fs;
use std::process::exit;

use img_parts::DynImage;

fn main() {
    let mut args = args();

    let input_path = match args.nth(1) {
        Some(path) => path,
        None => {
            eprintln!("Please specify the input file path (must be a jpeg, png or webp)");
            exit(1);
        }
    };

    let buf = fs::read(input_path).expect("image read");
    let dimg = DynImage::from_bytes(buf.into())
        .expect("image decoded")
        .expect("image format supported");

    match dimg {
        DynImage::Jpeg(jpeg) => {
            println!("---------------------------------");
            println!(
                "| {: <3} | {: <6} | {: <12} |",
                "i", "marker", "total length"
            );
            println!("---------------------------------");

            for (i, segment) in jpeg.segments().iter().enumerate() {
                // marker printed in HEX
                let marker = format!("{:X}", segment.marker());
                let len = segment.len_with_entropy();
                println!("| {: <3} | {: <6} | {: <12} |", i, marker, len);
            }
        }
        DynImage::Png(png) => {
            println!("---------------------------------");
            println!("| {: <3} | {: <6} | {: <12} |", "i", "type", "total length");
            println!("---------------------------------");

            for (i, chunk) in png.chunks().iter().enumerate() {
                let kind = chunk.kind();
                let kind = String::from_utf8_lossy(&kind);
                let len = chunk.len();
                println!("| {: <3} | {: <6} | {: <12} |", i, kind, len);
            }
        }
        DynImage::WebP(webp) => {
            println!("---------------------------------");
            println!("| {: <3} | {: <6} | {: <12} |", "i", "id", "total length");
            println!("---------------------------------");

            for (i, chunk) in webp.chunks().iter().enumerate() {
                let id = chunk.id();
                let id = String::from_utf8_lossy(&id).replace(0 as char, " ");
                let len = chunk.len();
                println!("| {: <3} | {: <6} | {: <12} |", i, id, len);
            }
        }
    }
}
