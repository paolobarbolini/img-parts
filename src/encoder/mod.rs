use std::io::{Result, Write};

use bytes::Bytes;

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
/// // this would also work with anything else from this crate that implements `encode`
/// let riff_chunk = RiffChunk::new([b'R', b'I', b'F', b'F'], RiffContent::Data(Bytes::new()));
///
/// let mut file = File::create("somefile.webp")?;
/// let encoder = riff_chunk.encode();
/// for chunk in encoder {
///     file.write_all(&chunk)?;
/// }
/// # Ok(())
/// # }
/// ```
///
/// This struct is created by the `encode` method on
///
/// * [`RiffChunk`][crate::riff::RiffContent::encode]
/// * [`RiffContent`][crate::riff::RiffContent::encode].
/// * [`WebP`][crate::webp::WebP::encode].
/// * [`Jpeg`][crate::jpeg::Jpeg::encode].
/// * [`JpegSegment`][crate::jpeg::JpegSegment::encode].
///
/// See their documentation for more.
#[derive(Clone)]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct ImageEncoder<I> {
    inner: I,
    pos: usize,
}

impl<I: EncodeAt> ImageEncoder<I> {
    /// Turns this `ImageEncoder` into a reader
    #[inline]
    pub fn read(self) -> ImageEncoderReader<I> {
        ImageEncoderReader::from(self)
    }

    /// Writes this `ImageEncoder` into a writer
    #[inline]
    pub fn write_to<W: Write>(self, writer: W) -> Result<u64> {
        self.read().write_to(writer)
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
