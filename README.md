# img-parts

[![CI](https://github.com/paolobarbolini/img-parts/workflows/CI/badge.svg)](https://github.com/paolobarbolini/img-parts/actions?query=workflow%3ACI)
[![Rustc Version 1.34.2+](https://img.shields.io/badge/rustc-1.34.2+-lightgray.svg)](https://blog.rust-lang.org/2019/04/11/Rust-1.34.0.html)

The `img-parts` crate provides a low level api for reading and
writing containers from various image formats.

It currently supports `Jpeg` and `RIFF` (with some helper functions
for `WebP`).

With it you can read an image, modify its sections and save it
back.

```rust,ignore
use img_parts::jpeg::Jpeg;
use img_parts::ImageICC;

let input = File::open("img.jpg")?;
let output = File::create("out.jpg")?;

let mut jpeg = Jpeg::read(&mut BufReader::new(input))?;
let icc_profile = jpeg.icc_profile();

jpeg.set_icc_profile(Some(another_icc_profile));
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
