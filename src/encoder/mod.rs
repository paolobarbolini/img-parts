use std::io::{Result, Write};

use bytes::{Bytes, BytesMut};

mod read;
pub use read::ImageEncoderReader;

/// An iterator that returns the [`Bytes`][bytes::Bytes] making up the inner image.
///
/// The image containers are composed of multiple [`Bytes`][bytes::Bytes] that can't be put together
/// without copying. If your usecase allows it use the `Iterator` in this `ImageEncoder` to stream
/// the contents to your output.
///
/// For example if you are saving the file to disk you could do it like so:
///
/// ```rust,no_run
/// # use std::io::{Result, Write};
/// # use std::fs::File;
/// # use bytes::Bytes;
/// # use img_parts::{ImageEncoder};
/// # use img_parts::riff::{RiffChunk, RiffContent};
/// # fn run() -> Result<()> {
/// // some RiffChunk just for this example
/// // this would also work with anything else from this crate that implements `encoder`
/// let riff_chunk = RiffChunk::new([b'R', b'I', b'F', b'F'], RiffContent::Data(Bytes::new()));
///
/// let mut file = File::create("somefile.webp")?;
/// let encoder = riff_chunk.encoder();
/// for chunk in encoder {
///     file.write_all(&chunk)?;
/// }
/// # Ok(())
/// # }
/// ```
///
/// This struct is created by the `encoder` method on
///
/// * [`RiffChunk`][crate::riff::RiffContent::encoder]
/// * [`RiffContent`][crate::riff::RiffContent::encoder].
/// * [`WebP`][crate::webp::WebP::encoder].
/// * [`Jpeg`][crate::jpeg::Jpeg::encoder].
/// * [`JpegSegment`][crate::jpeg::JpegSegment::encoder].
///
/// See their documentation for more.
#[derive(Clone)]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct ImageEncoder<I> {
    inner: I,
    pos: usize,
}

impl<I: EncodeAt> ImageEncoder<I> {
    /// Turns this `ImageEncoder` into a reader that will never fail
    #[inline]
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
    #[inline]
    pub fn write_to<W: Write>(self, writer: W) -> Result<u64> {
        self.read().write_to(writer)
    }

    /// Takes the pieces composing this `ImageEncoder` and copies
    /// them into a linear piece of memory.
    ///
    /// If possible [`write_to`][Self::write_to] should be used instead,
    /// since it avoids creating a copy of the entire file.
    pub fn bytes(self) -> Bytes {
        let mut bytes = BytesMut::new();
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
