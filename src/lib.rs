#![doc(html_root_url = "https://docs.rs/img-parts/0.1.0")]

//! # img-parts
//!
//! The `img-parts` crate provides a low level api for reading and
//! writing containers from various image formats.
//!
//! It currently supports [`Jpeg`][crate::jpeg::Jpeg] and
//! [`RIFF`][crate::riff::RiffChunk] (with some helper functions
//! for [`WebP`][crate::webp::WebP]).
//!
//! With it you can read an image, modify its sections and save it
//! back.
//!
//! ```rust,no_run
//! # use std::fs::File;
//! # use std::io::{BufReader, BufWriter};
//! # use img_parts::Result;
//! # fn run() -> Result<()> {
//! use img_parts::jpeg::Jpeg;
//! use img_parts::ImageICC;
//!
//! # let another_icc_profile = Vec::new();
//! let input = File::open("img.jpg")?;
//! let output = File::create("out.jpg")?;
//!
//! let mut jpeg = Jpeg::read(&mut BufReader::new(input))?;
//! let icc_profile = jpeg.icc_profile();
//!
//! jpeg.set_icc_profile(Some(another_icc_profile));
//! jpeg.write_to(&mut BufWriter::new(output))?;
//! # Ok(())
//! # }
//! ```

pub use common::ImageICC;
pub use error::{Error, Result};

mod common;
mod error;
pub mod jpeg;
pub mod riff;
pub mod vp8;
pub mod webp;
