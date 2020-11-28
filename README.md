# img-parts

[![crates.io](https://img.shields.io/crates/v/img-parts.svg)](https://crates.io/crates/img-parts)
[![Documentation](https://docs.rs/img-parts/badge.svg)](https://docs.rs/img-parts)
[![dependency status](https://deps.rs/crate/img-parts/0.2.3/status.svg)](https://deps.rs/crate/img-parts/0.2.3)
[![Rustc Version 1.39.0+](https://img.shields.io/badge/rustc-1.39.0+-lightgray.svg)](https://blog.rust-lang.org/2019/11/07/Rust-1.39.0.html)
[![CI](https://github.com/paolobarbolini/img-parts/workflows/CI/badge.svg)](https://github.com/paolobarbolini/img-parts/actions?query=workflow%3ACI)

The `img-parts` crate provides a low level API for reading and
writing containers from various image formats, and a high level
API for reading and writing raw ICC profiles and EXIF metadata.

It currently supports `Jpeg`, `Png` and `RIFF` (with some helper
functions for `WebP`).

More examples can be found in the `examples` directory on GitHub.

## Reading and writing raw ICCP and EXIF metadata

```rust,ignore
use std::fs::{self, File};

use img_parts::jpeg::Jpeg;
use img_parts::{ImageEXIF, ImageICC};

let input = fs::read("img.jpg")?;
let output = File::create("out.jpg")?;

let mut jpeg = Jpeg::from_bytes(input.into())?;
let icc_profile = jpeg.icc_profile();
let exif_metadata = jpeg.exif();

jpeg.set_icc_profile(Some(another_icc_profile.into()));
jpeg.set_exif(Some(new_exif_metadata.into()));
jpeg.encoder().write_to(output)?;
```

## Modifying chunks

```rust,no_run
use std::fs::{self, File};

use img_parts::jpeg::{markers, Jpeg, JpegSegment};
use img_parts::Bytes;

let input = fs::read("img.jpg")?;
let output = File::create("out.jpg")?;

let mut jpeg = Jpeg::from_bytes(input.into())?;

let comment = Bytes::from("Hello, I'm writing a comment!");
let comment_segment = JpegSegment::new_with_contents(markers::COM, comment);
jpeg.segments_mut().insert(1, comment_segment);

jpeg.encoder().write_to(output)?;
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
