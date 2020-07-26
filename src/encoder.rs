use bytes::Bytes;

/// An iterator that returns the [`Bytes`][bytes::Bytes] making up the inner image.
///
/// This struct is created by the `encode` method on
///
/// * [`RiffChunk`][crate::riff::RiffContent::encode]
/// * [`RiffContent`][crate::riff::RiffContent::encode].
/// * [`WebP`][crate::webp::WebP::encode].
/// * [`JpegSegment`][crate::jpeg::JpegSegment::encode].
///
/// See their documentation for more.
#[derive(Clone)]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct ImageEncoder<I> {
    inner: I,
    pos: usize,
}

impl<I> ImageEncoder<I> {
    pub(crate) fn new(inner: I) -> Self {
        Self { inner, pos: 0 }
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

pub trait EncodeAt {
    fn encode_at(&self, pos: &mut usize) -> Option<Bytes>;
}
