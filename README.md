# img-parts

[![crates.io](https://img.shields.io/crates/v/img-parts.svg)](https://crates.io/crates/img-parts)
[![Documentation](https://docs.rs/img-parts/badge.svg)](https://docs.rs/img-parts)
[![Rustc Version 1.34.2+](https://img.shields.io/badge/rustc-1.34.2+-lightgray.svg)](https://blog.rust-lang.org/2019/04/11/Rust-1.34.0.html)
[![CI](https://github.com/paolobarbolini/img-parts/workflows/CI/badge.svg)](https://github.com/paolobarbolini/img-parts/actions?query=workflow%3ACI)

The `img-parts` crate provides a low level api for reading and
writing containers from various image formats.

It currently supports `Jpeg` and `RIFF` (with some helper functions
for `WebP`).

With it you can read an image, modify its sections and save it
back.

```rust,ignore
use img_parts::jpeg::Jpeg;
use img_parts::{ImageEXIF, ImageICC};

let input = fs::read("img.jpg")?;
let output = File::create("out.jpg")?;

let mut jpeg = Jpeg::from_bytes(input.into())?;
let icc_profile = jpeg.icc_profile();
let exif_metadata = jpeg.exif();

jpeg.set_icc_profile(Some(another_icc_profile.into()));
jpeg.set_exif(Some(new_exif_metadata.into()));
jpeg.write_to(&mut BufWriter::new(output))?;
```

## License

Licensed under either of
 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as above, without any
additional terms or conditions.
