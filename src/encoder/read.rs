use core::mem;
use std::io::{Read, Result};

use bytes::Bytes;

use super::{EncodeAt, ImageEncoder};

/// A reader for `ImageEncoder` that will never fail
///
/// This struct is created by the [`read`][ImageEncoder::read] method on [`ImageEncoder`][ImageEncoder]
#[derive(Debug)]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub struct ImageEncoderReader<I> {
    inner: ImageEncoder<I>,
    buf: Bytes,
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

#[cfg(test)]
mod tests {
    use super::{EncodeAt, ImageEncoder};
    use bytes::Bytes;
    use std::io::Read;

    struct FakeEncodeAt {
        vec: Vec<Bytes>,
    }

    impl EncodeAt for FakeEncodeAt {
        fn encode_at(&self, pos: &mut usize) -> Option<Bytes> {
            self.vec.get(*pos).cloned()
        }

        fn len(&self) -> usize {
            self.vec.iter().map(|buf| buf.len()).sum()
        }
    }

    #[test]
    fn image_encoder_reader_read() {
        let encoder_at = FakeEncodeAt {
            vec: vec![
                Bytes::from_static(b"abcd"),
                Bytes::from_static(b"9876"),
                Bytes::from_static(b"ducks!"),
            ],
        };
        let encoder = ImageEncoder::from(encoder_at);
        let mut reader = encoder.read();

        let mut r = [0u8; 2];
        assert_eq!(reader.read(&mut r).unwrap(), 2);
        assert_eq!(&r, b"ab");

        let mut r = [0u8; 1];
        assert_eq!(reader.read(&mut r).unwrap(), 1);
        assert_eq!(&r, b"c");

        let mut r = [0u8; 8];
        assert_eq!(reader.read(&mut r).unwrap(), 1);
        assert_eq!(&r[..1], b"d");

        let mut r = [0u8; 8];
        assert_eq!(reader.read(&mut r).unwrap(), 4);
        assert_eq!(&r[..4], b"9876");

        let mut r = [0u8; 4];
        assert_eq!(reader.read(&mut r).unwrap(), 4);
        assert_eq!(&r[..4], b"duck");

        let mut r = [0u8; 4];
        assert_eq!(reader.read(&mut r).unwrap(), 2);
        assert_eq!(&r[..2], b"s!");

        for _ in 0..3 {
            let mut r = [0u8; 4];
            assert_eq!(reader.read(&mut r).unwrap(), 0);
            assert_eq!(&r[..], &[0, 0, 0, 0]);
        }
    }
}
