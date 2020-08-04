use std::io::{Read, Result, Write};
use std::mem;

use bytes::Bytes;

use super::{EncodeAt, ImageEncoder};

/// A reader for `ImageEncoder`
///
/// This struct is created by the [`read`][ImageEncoder::read] method on [`ImageEncoder`][ImageEncoder]
pub struct ImageEncoderReader<I> {
    inner: ImageEncoder<I>,
    buf: Bytes,
}

impl<I: EncodeAt> ImageEncoderReader<I> {
    /// Writes this `ImageEncoderReader` into a writer
    ///
    /// Returns the number of bytes written.
    ///
    /// # Errors
    ///
    /// This methods fails if writing fails.
    #[inline]
    pub fn write_to<W: Write>(self, mut writer: W) -> Result<u64> {
        let mut len = 0;

        for chunk in self.inner {
            len += chunk.len() as u64;
            writer.write_all(&chunk)?;
        }

        Ok(len)
    }
}

impl<I: EncodeAt> Read for ImageEncoderReader<I> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        if self.buf.is_empty() {
            match self.inner.next() {
                Some(next) => self.buf = next,
                None => return Ok(0),
            };
        }

        let to_read = if buf.len() > self.buf.len() {
            // TODO: replace this with mem::take once 1.40 is our MSRV
            mem::replace(&mut self.buf, Bytes::new())
        } else {
            self.buf.split_to(buf.len())
        };

        let len = to_read.len();
        buf[..len].copy_from_slice(&to_read);
        Ok(len)
    }
}

impl<I> From<ImageEncoder<I>> for ImageEncoderReader<I> {
    fn from(ie: ImageEncoder<I>) -> ImageEncoderReader<I> {
        Self {
            inner: ie,
            buf: Bytes::new(),
        }
    }
}
