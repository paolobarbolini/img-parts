//! # img-parts
//!
//! The `img-parts` crate provides a low level api for reading and
//! writing containers from various image formats.
//!
//! It currently supports [`Jpeg`][crate::jpeg::Jpeg],
//! [`Png`][crate::png::Png] and [`RIFF`][crate::riff::RiffChunk]
//! (with some helper functions for [`WebP`][crate::webp::WebP]).
//!
//! ## Reading and writing raw ICCP and EXIF metadata
//!
//! ```rust,no_run
//! # use std::result::Result;
//! # use std::error::Error;
//! # #[cfg(feature = "std")]
//! # fn run() -> Result<(), Box<dyn Error + 'static>> {
//! use std::fs::{self, File};
//!
//! use img_parts::jpeg::Jpeg;
//! use img_parts::{ImageEXIF, ImageICC};
//!
//! # let another_icc_profile = Vec::new();
//! # let new_exif_metadata = Vec::new();
//! let input = fs::read("img.jpg")?;
//! let output = File::create("out.jpg")?;
//!
//! let mut jpeg = Jpeg::from_bytes(input.into())?;
//! let icc_profile = jpeg.icc_profile();
//! let exif_metadata = jpeg.exif();
//!
//! jpeg.set_icc_profile(Some(another_icc_profile.into()));
//! jpeg.set_exif(Some(new_exif_metadata.into()));
//! jpeg.encoder().write_to(output)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Modifying chunks
//!
//! ```rust,no_run
//! # use std::result::Result;
//! # use std::error::Error;
//! # #[cfg(feature = "std")]
//! # fn run() -> Result<(), Box<dyn Error + 'static>> {
//! use std::fs::{self, File};
//!
//! use img_parts::jpeg::{markers, Jpeg, JpegSegment};
//! use img_parts::Bytes;
//!
//! let input = fs::read("img.jpg")?;
//! let output = File::create("out.jpg")?;
//!
//! let mut jpeg = Jpeg::from_bytes(input.into())?;
//!
//! let comment = Bytes::from("Hello, I'm writing a comment!");
//! let comment_segment = JpegSegment::new_with_contents(markers::COM, comment);
//! jpeg.segments_mut().insert(1, comment_segment);
//!
//! jpeg.encoder().write_to(output)?;
//! # Ok(())
//! # }
//! ```

#![deny(missing_debug_implementations)]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]
// requires Rust >=1.42
#![allow(clippy::match_like_matches_macro)]

extern crate alloc;

pub use bytes::Bytes;

pub use common::DynImage;
pub use encoder::ImageEncoder;
#[cfg(feature = "std")]
pub use encoder::ImageEncoderReader;
pub use error::{Error, Result};
pub use traits::{ImageEXIF, ImageICC};

pub(crate) const EXIF_DATA_PREFIX: &[u8] = b"Exif\0\0";

mod common;
mod encoder;
mod error;
pub mod jpeg;
pub mod png;
pub mod riff;
mod traits;
pub(crate) mod util;
pub mod vp8;
pub mod webp;
