## [0.2.0]

* Added support for `Png`.
* Added `DynImage`, an enum that wraps `Jpeg`, `Png` and `WebP` to give easy
  access to decoding, encoding and the `ImageICC` and `ImageEXIF` traits
  without having to write separate code for every format.
* Replaced internal usage of `Vec<u8>` with [`Bytes`][bytes05] from the
  `bytes` crate, allowing more efficient memory managment.
* Replaced `Jpeg::read` and `WebP::read` with `Jpeg::from_bytes`
  and `WebP::from_bytes`.
* Deprecated `Jpeg::write_to` and `WebP::write_to`. The new way of encoding
  an image is to call the `encoder` method, which will return an `ImageEncoder`
  that can write the file piece by piece, return every piece through an
  `Iterator`, or copy every piece into a linear piece of memory and returning it.
* Tidied up and wrote more meaninful errors in the `Error` enum.
* Implemented `std::error::Error` for the `Error` enum.
* Bumped the MSRV to 1.39.0.
* Removed the `byteorder` crate.
* Improved the documentation and added examples.

[bytes05]: https://docs.rs/bytes/0.5/bytes/struct.Bytes.html
[0.2.0]: https://github.com/paolobarbolini/img-parts/compare/v0.1.1...v0.2.0

## [0.1.1]

* EXIF extraction via the `ImageEXIF` trait.
* Simplified `Jpeg` `Entropy` decoding, removing the `bytecount` crate.

[0.1.1]: https://github.com/paolobarbolini/img-parts/compare/v0.1.0...v0.1.1

## 0.1.0

* `Jpeg` and `WebP` read and write support.
* ICC extraction via the `ImageICC` trait.
