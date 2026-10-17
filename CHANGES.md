## [0.4.1]

* Fix panics with malformed images in `WebP::dimensions`, `Png::icc_profile`, `Jpeg::icc_profile` and the `set_icc_profile` and `set_exif` methods
* Fix stack overflow when parsing deeply nested RIFF lists
* Limit the decompressed size of PNG ICC profiles to 16 MiB
* Fix `WebP::dimensions` returning wrong values for images with a VP8X chunk
* Fix WebP images with alpha, animation, XMP metadata or unknown chunks, or lossless with an ICC profile, becoming invalid when changing their ICC profile or EXIF metadata
* Fix WebP VP8X flags not matching the chunks after changing the ICC profile or EXIF metadata, making readers ignore the metadata or reject the image
* Insert PNG eXIf chunks before the image data
* Remove the PNG sRGB chunk when setting an ICC profile
* Insert JPEG ICC profile and EXIF segments before the frame header
* Replace JPEG EXIF segments in place, and insert new ones right after the SOI marker and any APP0 segments, like the JFIF one, so that readers find them
* Write multi-segment JPEG ICC profiles in sequence order
* Discard JPEG EXIF metadata and ICC profiles too big to be stored, instead of panicking or writing a corrupted profile
* Ignore JPEG ICC profiles with missing, duplicated or inconsistent segments
* Keep the offsets in the MPF segment of JPEG images valid when changing their ICC profile or EXIF metadata, so that their other pictures stay readable
* Keep the EOI marker of JPEG images without an SOS segment
* Fix encoding of JPEG markers without a length, and parsing of RSTn, DNL, DHP, EXP and JPGn markers
* Skip JPEG markers without a length when parsing, instead of keeping each of them as a segment
* Fix `ImageEncoderReader` stopping early at empty chunks, truncating every PNG image
* Fix encoding of JPEG segments with entropy coded data but no contents
* Encode images in linear time, instead of quadratic in the number of chunks
* Reduce the memory needed to parse RIFF lists
* Bump miniz_oxide crate to 0.9
* Bump minimum bytes crate version to 1.10
* Bump minimum crc32fast crate version to 1.2.1

[0.4.1]: https://github.com/paolobarbolini/img-parts/compare/v0.4.0...v0.4.1

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
