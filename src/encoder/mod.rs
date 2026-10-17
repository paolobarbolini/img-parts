use alloc::vec::{self, Vec};
use core::fmt;
#[cfg(feature = "std")]
use std::io::{self, Write};

use bytes::{Bytes, BytesMut};

#[cfg(feature = "std")]
mod read;
#[cfg(feature = "std")]
pub use read::ImageEncoderReader;

/// A streaming encoder for the binary representation of an image.
///
/// Image data is composed of multiple chunks held in separate memory
/// locations. This encoder provides a memory-efficient, mostly zero-copy
/// way of encoding these fragmented chunks. Streaming the chunks one
/// by one avoids the need to allocate a single, large contiguous
/// buffer and the cost of having to memcpy the data into the buffer.
///
/// This can be done in the following ways:
///
/// - `Iterator`: Iterate through each chunk individually.
///   This is the lowest level and most powerful API.
/// - `io::Write`: The [`write_to`][ImageEncoder::write_to] method provides a
///   convenient way to write all chunks sequentially to any `std::io::Write`
///   target, like a file or a network socket.
/// - `Bytes`: The [`bytes`][ImageEncoder::bytes] method is
///   available for cases requiring a single, contiguous byte buffer. It copies
///   all chunks into a new [`Bytes`].
///
/// ## Saving a file to disk
///
/// ```rust,no_run
/// # use std::io::{Result, Write};
/// # use std::fs::File;
/// # use bytes::Bytes;
/// # use img_parts::{ImageEncoder};
/// # use img_parts::riff::{RiffChunk, RiffContent};
/// # #[cfg(feature = "std")]
/// # fn run() -> Result<()> {
/// // some RiffChunk just for this example
/// // this would also work with anything else from this crate that implements `encoder`
/// let riff_chunk = RiffChunk::new(*b"RIFF", RiffContent::Data(Bytes::new()));
///
/// let file = File::create("somefile.webp")?;
/// riff_chunk.encoder().write_to(file)?;
/// # Ok(())
/// # }
/// ```
///
/// This struct is created by the `encoder` method on
///
/// * [`RiffChunk`][crate::riff::RiffChunk::encoder]
/// * [`RiffContent`][crate::riff::RiffContent::encoder]
/// * [`WebP`][crate::webp::WebP::encoder]
/// * [`Jpeg`][crate::jpeg::Jpeg::encoder]
/// * [`JpegSegment`][crate::jpeg::JpegSegment::encoder]
/// * [`Png`][crate::png::Png::encoder]
/// * [`PngChunk`][crate::png::PngChunk::encoder]
/// * [`DynImage`][crate::DynImage::encoder]
///
/// See their documentation for more.
#[derive(Clone)]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct ImageEncoder<I> {
    inner: I,
    pieces: vec::IntoIter<Bytes>,
}

impl<I: Encode> ImageEncoder<I> {
    /// Turns this `ImageEncoder` into a reader that will never fail
    #[inline]
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    pub fn read(self) -> ImageEncoderReader<I> {
        ImageEncoderReader::from(self)
    }

    /// Writes this `ImageEncoder` into a writer
    ///
    /// Returns the number of bytes written.
    ///
    /// # Errors
    ///
    /// This methods fails if writing fails.
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    pub fn write_to<W: Write>(self, mut writer: W) -> io::Result<u64> {
        let mut len = 0;

        for chunk in self {
            len += chunk.len() as u64;
            writer.write_all(&chunk)?;
        }

        Ok(len)
    }

    /// Returns the entire `ImageEncoder` in a single linear piece of memory
    ///
    /// Takes the pieces composing this `ImageEncoder` and copies
    /// them into a linear piece of memory.
    ///
    /// If possible [`write_to`][Self::write_to] should be used instead,
    /// since it avoids creating a second in memory copy of the file.
    pub fn bytes(self) -> Bytes {
        let len = self.pieces.as_slice().iter().map(Bytes::len).sum();
        let mut bytes = BytesMut::with_capacity(len);
        for piece in self {
            bytes.extend_from_slice(&piece);
        }

        bytes.freeze()
    }
}

impl<I: Encode> Iterator for ImageEncoder<I> {
    type Item = Bytes;

    fn next(&mut self) -> Option<Self::Item> {
        self.pieces.next()
    }
}

impl<I: Encode> From<I> for ImageEncoder<I> {
    fn from(inner: I) -> ImageEncoder<I> {
        let mut pieces = Vec::new();
        inner.encode(&mut pieces);

        Self {
            inner,
            pieces: pieces.into_iter(),
        }
    }
}

impl<I: fmt::Debug> fmt::Debug for ImageEncoder<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImageEncoder")
            .field("inner", &self.inner)
            .finish_non_exhaustive()
    }
}

pub trait Encode {
    /// Appends the pieces making up the encoded representation to `pieces`
    fn encode(&self, pieces: &mut Vec<Bytes>);
}

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};
    #[cfg(feature = "std")]
    use std::io::Read;

    use bytes::Bytes;

    use super::{Encode, ImageEncoder};

    struct FakeEncode {
        vec: Vec<Bytes>,
    }

    impl Encode for FakeEncode {
        fn encode(&self, pieces: &mut Vec<Bytes>) {
            pieces.extend(self.vec.iter().cloned());
        }
    }

    #[test]
    fn image_encoder_iter() {
        let fake = FakeEncode {
            vec: vec![
                Bytes::from_static(b"abcd"),
                Bytes::from_static(b"9876"),
                Bytes::from_static(b"ducks!"),
            ],
        };
        let mut encoder = ImageEncoder::from(fake);

        assert_eq!(Some(Bytes::from_static(b"abcd")), encoder.next());
        assert_eq!(Some(Bytes::from_static(b"9876")), encoder.next());
        assert_eq!(Some(Bytes::from_static(b"ducks!")), encoder.next());
        assert!(encoder.next().is_none());
    }

    #[test]
    #[cfg(feature = "std")]
    fn image_encoder_read() {
        let fake = FakeEncode {
            vec: vec![
                Bytes::from_static(b"abcd"),
                Bytes::from_static(b"9876"),
                Bytes::from_static(b"duck!"),
            ],
        };
        let encoder = ImageEncoder::from(fake);
        let mut reader = encoder.read();

        let expected = [
            Bytes::from_static(b"abcd"),
            Bytes::from_static(b"9876"),
            Bytes::from_static(b"duck!"),
        ];

        for exp in &expected {
            let mut buf = [0; 32];
            let read = reader.read(&mut buf).expect("read something");

            assert_eq!(exp, &buf[..read]);
        }

        let mut buf = [0; 32];
        let read = reader.read(&mut buf).expect("read nothing");
        assert_eq!(0, read);
        assert_eq!([0; 32], buf);
    }

    #[test]
    #[cfg(feature = "std")]
    fn image_encoder_read_buffered() {
        let fake = FakeEncode {
            vec: vec![
                Bytes::from_static(b"abcd"),
                Bytes::from_static(b"9876"),
                Bytes::from_static(b"duck!"),
            ],
        };
        let encoder = ImageEncoder::from(fake);
        let mut reader = encoder.read();

        let expected = [
            Bytes::from_static(b"ab"),
            Bytes::from_static(b"cd"),
            Bytes::from_static(b"98"),
            Bytes::from_static(b"76"),
            Bytes::from_static(b"du"),
            Bytes::from_static(b"ck"),
            Bytes::from_static(b"!"),
        ];

        for exp in &expected {
            let mut buf = [0; 2];
            let read = reader.read(&mut buf).expect("read something");

            assert_eq!(exp, &buf[..read]);
        }

        let mut buf = [0; 32];
        let read = reader.read(&mut buf).expect("read nothing");
        assert_eq!(0, read);
        assert_eq!([0; 32], buf);
    }

    #[test]
    #[cfg(feature = "std")]
    fn image_encoder_write_to() {
        let fake = FakeEncode {
            vec: vec![
                Bytes::from_static(b"abcd"),
                Bytes::from_static(b"9876"),
                Bytes::from_static(b"duck!"),
            ],
        };
        let encoder = ImageEncoder::from(fake);

        let mut vec = Vec::new();
        let written = encoder.write_to(&mut vec).expect("write_to");
        assert_eq!(written, 13);
        assert_eq!(vec, b"abcd9876duck!");
    }
}
