## [0.4.0]

* Stop prepending JPEG preamble to WebP EXIF chunks and support decoding chunks with no preamble
* Bump MSRV to 1.63

[0.4.0]: https://github.com/paolobarbolini/img-parts/compare/v0.3.3...v0.4.0

## [0.3.3]

* Support reading VP8L dimensions

[0.3.3]: https://github.com/paolobarbolini/img-parts/compare/v0.3.2...v0.3.3

## [0.3.2]

* Fix encoder underflow with malformed JPEGs

[0.3.2]: https://github.com/paolobarbolini/img-parts/compare/v0.3.1...v0.3.2

## [0.3.1]

* Bump MSRV to 1.57
* Bump edition to 2021
* Bump miniz_oxide crate to 0.8
* Make license metadata SPDX compliant

[0.3.1]: https://github.com/paolobarbolini/img-parts/compare/v0.3.0...v0.3.1

## [0.3.0]

* Ignore remaining PNG data after IEND chunk
* Bump bytes crate to 1.0

[0.3.0]: https://github.com/paolobarbolini/img-parts/compare/v0.2.3...v0.3.0

## [0.2.3]

* Fix decoding JPEGs containing a DRI segment

[0.2.3]: https://github.com/paolobarbolini/img-parts/compare/v0.2.2...v0.2.3

## [0.2.2]

* Fix potential integer overflow when calculating jpeg segment size

[0.2.2]: https://github.com/paolobarbolini/img-parts/compare/v0.2.1...v0.2.2

## [0.2.1]

* Fix docs.rs build

[0.2.1]: https://github.com/paolobarbolini/img-parts/compare/v0.2.0...v0.2.1

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
* Added `no_std` support via the `std` feature, which is enabled by default.
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
