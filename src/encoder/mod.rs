#[cfg(feature = "std")]
use std::io::{self, Write};

use bytes::{Bytes, BytesMut};

#[cfg(feature = "std")]
mod read;
#[cfg(feature = "std")]
pub use read::ImageEncoderReader;

/// An encoder for and image container or for an image chunk
///
/// As image containers contain multiple _chunks_, each one stored as a separate piece of memory,
/// the representation of the entire file is fragmented in memory.
/// This crate tries to be as efficient as possible with memory, by giving access to
/// the underlying fragmented chunks representing the full file, which can then be written or
/// streamed one at at time.
///
/// Those chunks can be accessed through an `Iterator`, similar to how
/// [futures::stream::Stream of Bytes][stream] are used in the futures crate.
/// A [`write_to`][ImageEncoder::write_to] method is also provided, which calls
/// [`Write::write_all`][write_all] for every chunk returned by the Iterator.
/// The [`bytes`][ImageEncoder::bytes] method is provided for cases where the image has to
/// fit into a linear piece of memory. In that case all of the chunks get copied into
/// a single linear piece of memory.
///
/// [write_all]: std::io::Write::write_all
/// [stream]: https://docs.rs/futures-core/0.3/futures_core/stream/trait.Stream.html
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
/// let riff_chunk = RiffChunk::new([b'R', b'I', b'F', b'F'], RiffContent::Data(Bytes::new()));
///
/// let file = File::create("somefile.webp")?;
/// riff_chunk.encoder().write_to(file);
/// # Ok(())
/// # }
/// ```
///
/// This struct is created by the `encoder` method on
///
/// * [`RiffChunk`][crate::riff::RiffContent::encoder]
/// * [`RiffContent`][crate::riff::RiffContent::encoder]
/// * [`WebP`][crate::webp::WebP::encoder]
/// * [`Jpeg`][crate::jpeg::Jpeg::encoder]
/// * [`JpegSegment`][crate::jpeg::JpegSegment::encoder]
/// * [`Png`][crate::png::Png::encoder]
/// * [`PngChunk`][crate::png::PngChunk::encoder]
///
/// See their documentation for more.
#[derive(Debug, Clone)]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct ImageEncoder<I> {
    inner: I,
    pos: usize,
}

impl<I: EncodeAt> ImageEncoder<I> {
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
        let mut bytes = BytesMut::with_capacity(self.inner.len());
        for piece in self {
            bytes.extend_from_slice(&piece);
        }

        bytes.freeze()
    }
}

impl<I: EncodeAt> Iterator for ImageEncoder<I> {
    type Item = Bytes;

    fn next(&mut self) -> Option<Self::Item> {
        let mut pos = self.pos;
        let item = self.inner.encode_at(&mut pos);

        if item.is_some() {
            self.pos += 1;
        }

        item
    }
}

impl<I: EncodeAt> From<I> for ImageEncoder<I> {
    fn from(ea: I) -> ImageEncoder<I> {
        Self { inner: ea, pos: 0 }
    }
}

pub trait EncodeAt {
    fn encode_at(&self, pos: &mut usize) -> Option<Bytes>;

    fn len(&self) -> usize;
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "std")]
    use std::io::Read;
    use alloc::vec::Vec;
    use alloc::vec;

    use super::{EncodeAt, ImageEncoder};
    use bytes::Bytes;

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
    fn image_encoder_iter() {
        let encoder_at = FakeEncodeAt {
            vec: vec![
                Bytes::from_static(b"abcd"),
                Bytes::from_static(b"9876"),
                Bytes::from_static(b"ducks!"),
            ],
        };
        let mut encoder = ImageEncoder::from(encoder_at);

        assert_eq!(Some(Bytes::from_static(b"abcd")), encoder.next());
        assert_eq!(Some(Bytes::from_static(b"9876")), encoder.next());
        assert_eq!(Some(Bytes::from_static(b"ducks!")), encoder.next());
        assert!(encoder.next().is_none());
    }

    #[test]
    #[cfg(feature = "std")]
    fn image_encoder_read() {
        let encoder_at = FakeEncodeAt {
            vec: vec![
                Bytes::from_static(b"abcd"),
                Bytes::from_static(b"9876"),
                Bytes::from_static(b"duck!"),
            ],
        };
        let encoder = ImageEncoder::from(encoder_at);
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
        let encoder_at = FakeEncodeAt {
            vec: vec![
                Bytes::from_static(b"abcd"),
                Bytes::from_static(b"9876"),
                Bytes::from_static(b"duck!"),
            ],
        };
        let encoder = ImageEncoder::from(encoder_at);
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
        let encoder_at = FakeEncodeAt {
            vec: vec![
                Bytes::from_static(b"abcd"),
                Bytes::from_static(b"9876"),
                Bytes::from_static(b"duck!"),
            ],
        };
        let encoder = ImageEncoder::from(encoder_at);

        let mut vec = Vec::new();
        let written = encoder.write_to(&mut vec).expect("write_to");
        assert_eq!(written, 13);
        assert_eq!(vec, b"abcd9876duck!");
    }
}
