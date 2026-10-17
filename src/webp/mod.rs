use alloc::vec::Vec;

use bytes::{Bytes, BytesMut};
use flags::WebPFlags;

use crate::{
    encoder::ImageEncoder,
    riff::{RiffChunk, RiffContent},
    util::{u24_from_le_bytes, u24_to_le_bytes},
    vp8::{size_from_vp8_header, VP8Kind},
    Error, ImageEXIF, ImageICC, Result, EXIF_DATA_PREFIX, EXIF_START_PREFIX_BE,
    EXIF_START_PREFIX_LE,
};

mod flags;

pub const CHUNK_ALPH: [u8; 4] = *b"ALPH";
pub const CHUNK_ANIM: [u8; 4] = *b"ANIM";
pub const CHUNK_ANMF: [u8; 4] = *b"ANMF";
pub const CHUNK_EXIF: [u8; 4] = *b"EXIF";
pub const CHUNK_ICCP: [u8; 4] = *b"ICCP";
pub const CHUNK_VP8: [u8; 4] = *b"VP8 ";
pub const CHUNK_VP8L: [u8; 4] = *b"VP8L";
pub const CHUNK_VP8X: [u8; 4] = *b"VP8X";
pub const CHUNK_XMP: [u8; 4] = *b"XMP ";

pub(crate) fn is_webp(buf: &[u8]) -> bool {
    buf.len() > 12 && &buf[..4] == b"RIFF" && &buf[8..12] == b"WEBP"
}

/// The representation of a WebP image
#[derive(Debug, Clone, PartialEq)]
pub struct WebP {
    riff: RiffChunk,
}

#[allow(clippy::len_without_is_empty)]
impl WebP {
    /// Construct a new `WebP` image from a [`RiffChunk`].
    ///
    /// # Errors
    ///
    /// This method returns a [`Error::WrongSignature`]
    /// if the content of the [`RiffChunk`] isn't a `List` or
    /// if the list's kind isn't "WEBP".
    pub fn new(riff: RiffChunk) -> Result<WebP> {
        match riff.content().list() {
            Some((kind, _)) if kind == Some(*b"WEBP") => Ok(WebP { riff }),
            _ => Err(Error::WrongSignature),
        }
    }

    /// Create a new `WebP` image from a Reader.
    ///
    /// # Errors
    ///
    /// This method fails if the file signature doesn't match or if
    /// it is corrupted or truncated.
    #[inline]
    pub fn from_bytes(b: Bytes) -> Result<WebP> {
        let riff = RiffChunk::from_bytes(b)?;
        WebP::new(riff)
    }

    /// Get the `VP8Kind` of this `WebP`.
    pub fn kind(&self) -> VP8Kind {
        if self.has_chunk(CHUNK_VP8X) {
            VP8Kind::VP8X
        } else if self.has_chunk(CHUNK_VP8L) {
            VP8Kind::VP8L
        } else {
            VP8Kind::VP8
        }
    }

    fn infer_kind(&self) -> VP8Kind {
        if self.has_chunk(CHUNK_ICCP) | self.has_chunk(CHUNK_EXIF) {
            VP8Kind::VP8X
        } else {
            // TODO: VP8L
            VP8Kind::VP8
        }
    }

    fn convert_into_infered_kind(&mut self) {
        let current_kind = self.kind();
        let correct_kind = self.infer_kind();

        if current_kind == correct_kind {
            if correct_kind == VP8Kind::VP8X {
                // TODO: update flags in the VP8X chunk
            }
        } else if correct_kind == VP8Kind::VP8 {
            self.remove_chunks_by_id(CHUNK_VP8X);
        } else if correct_kind == VP8Kind::VP8X {
            // TODO VP8L

            let pos = self
                .chunks()
                .iter()
                .position(|chunk| chunk.id() == CHUNK_ICCP)
                .unwrap_or(0);

            // the VP8X chunk can't be created without knowing the canvas size
            let (width, height) = match self.dimensions() {
                Some(dimensions) => dimensions,
                None => return,
            };

            let flags = WebPFlags::from_webp(self);
            let mut content = BytesMut::with_capacity(10);

            content.extend_from_slice(&flags.0);

            let buf = u24_to_le_bytes(width - 1);
            content.extend_from_slice(&buf);
            let buf = u24_to_le_bytes(height - 1);
            content.extend_from_slice(&buf);

            let chunk = RiffChunk::new(CHUNK_VP8X, RiffContent::Data(content.freeze()));
            self.chunks_mut().insert(pos, chunk);
        }
    }

    /// Get the width and height of this `WebP`.
    ///
    /// If this `WebP` has a `VP8X` chunk the dimension is the canvas size.
    ///
    /// Otherwise the dimension is read from the VP8 bitstream header.
    ///
    /// Returns `None` if the dimensions can't be read.
    pub fn dimensions(&self) -> Option<(u32, u32)> {
        if let Some(vp8x) = self.chunk_by_id(CHUNK_VP8X) {
            if let Some(data) = vp8x.content().data() {
                if let Some(range) = data.get(2..8) {
                    let width = u24_from_le_bytes(range[0..3].try_into().unwrap()) + 1;
                    let height = u24_from_le_bytes(range[3..6].try_into().unwrap()) + 1;
                    return Some((width, height));
                }
            }
        }

        if let Some(vp8) = self.chunk_by_id(CHUNK_VP8) {
            if let Some(data) = vp8.content().data() {
                if let Some((width, height)) = size_from_vp8_header(data) {
                    return Some((width as u32, height as u32));
                }
            }
        }

        if let Some(vp8l) = self.chunk_by_id(CHUNK_VP8L) {
            if let Some(data) = vp8l.content().data() {
                if let Some(range) = data.get(1..5) {
                    let size = u32::from_le_bytes(range.try_into().unwrap());
                    let width = (size & 0x3FFF) + 1;
                    let height = ((size >> 14) & 0x3FFF) + 1;

                    return Some((width, height));
                }
            }
        }

        None
    }

    /// Get the chunks of this `WebP`.
    pub fn chunks(&self) -> &Vec<RiffChunk> {
        match self.riff.content() {
            RiffContent::List { subchunks, .. } => subchunks,
            _ => unreachable!(),
        }
    }

    /// Get a mutable reference to the chunks of this `WebP`.
    pub fn chunks_mut(&mut self) -> &mut Vec<RiffChunk> {
        match self.riff.content_mut() {
            RiffContent::List {
                ref mut subchunks, ..
            } => subchunks,
            _ => unreachable!(),
        }
    }

    /// Check if there's a chunk with an id of `id`.
    #[inline]
    pub fn has_chunk(&self, id: [u8; 4]) -> bool {
        self.chunk_by_id(id).is_some()
    }

    /// Get the first chunk with an id of `id`.
    pub fn chunk_by_id(&self, id: [u8; 4]) -> Option<&RiffChunk> {
        self.chunks().iter().find(|chunk| chunk.id() == id)
    }

    /// Get every chunk with an id of `id`.
    pub fn chunks_by_id(&self, id: [u8; 4]) -> impl Iterator<Item = &RiffChunk> {
        self.chunks().iter().filter(move |chunk| chunk.id() == id)
    }

    /// Remove every chunk with an id of `id`
    pub fn remove_chunks_by_id(&mut self, id: [u8; 4]) {
        self.chunks_mut().retain(|chunk| chunk.id() != id);
    }

    /// Get the total size of the `WebP` once it is encoded.
    ///
    /// Internally calls [`RiffChunk::len`][crate::riff::RiffChunk::len] on the
    /// inner `RiffChunk`
    #[inline]
    pub fn len(&self) -> u32 {
        self.riff.len()
    }

    /// Create an [encoder][crate::ImageEncoder] for this `WebP`
    ///
    /// Internally calls [`RiffChunk::encoder`][crate::riff::RiffChunk::encoder] on the
    /// inner `RiffChunk`
    #[inline]
    pub fn encoder(self) -> ImageEncoder<RiffChunk> {
        self.riff.encoder()
    }

    pub(crate) fn inner(&self) -> &RiffChunk {
        &self.riff
    }
}

impl ImageICC for WebP {
    fn icc_profile(&self) -> Option<Bytes> {
        Some(self.chunk_by_id(CHUNK_ICCP)?.content().data()?.clone())
    }

    fn set_icc_profile(&mut self, profile: Option<Bytes>) {
        self.remove_chunks_by_id(CHUNK_ICCP);

        if let Some(profile) = profile {
            let kind = self.infer_kind();
            let pos = match kind {
                VP8Kind::VP8 => self
                    .chunks()
                    .iter()
                    .position(|chunk| chunk.id() == CHUNK_VP8),
                VP8Kind::VP8L | VP8Kind::VP8X => self
                    .chunks()
                    .iter()
                    .position(|chunk| chunk.id() == CHUNK_VP8L || chunk.id() == CHUNK_VP8X)
                    .map(|pos| pos + 1),
            }
            .unwrap_or(0);

            let chunk = RiffChunk::new(CHUNK_ICCP, RiffContent::Data(profile));
            self.chunks_mut().insert(pos, chunk);
        }

        self.convert_into_infered_kind();
    }
}

impl ImageEXIF for WebP {
    fn exif(&self) -> Option<Bytes> {
        const PREFIX_LEN: usize = EXIF_DATA_PREFIX.len();
        let data = self.chunk_by_id(CHUNK_EXIF)?.content().data()?;

        let raw_exif_offset =
            if data.starts_with(EXIF_START_PREFIX_LE) || data.starts_with(EXIF_START_PREFIX_BE) {
                Some(0)
            } else if data.starts_with(&[EXIF_DATA_PREFIX, EXIF_START_PREFIX_LE].concat())
                || data.starts_with(&[EXIF_DATA_PREFIX, EXIF_START_PREFIX_BE].concat())
            {
                Some(PREFIX_LEN)
            } else {
                None
            };

        raw_exif_offset.map(|offs| data.slice(offs..))
    }

    fn set_exif(&mut self, exif: Option<Bytes>) {
        self.remove_chunks_by_id(CHUNK_EXIF);

        if let Some(exif) = exif {
            let chunk = RiffChunk::new(CHUNK_EXIF, RiffContent::Data(exif));
            self.chunks_mut().push(chunk);
        }

        self.convert_into_infered_kind();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from_chunks(chunks: &[([u8; 4], &'static [u8])]) -> WebP {
        let subchunks = chunks
            .iter()
            .map(|&(id, data)| RiffChunk::new(id, RiffContent::Data(Bytes::from_static(data))))
            .collect();
        let riff = RiffChunk::new(
            *b"RIFF",
            RiffContent::List {
                kind: Some(*b"WEBP"),
                subchunks,
            },
        );
        WebP::new(riff).unwrap()
    }

    #[test]
    fn vp8_dimensions() {
        let webp = from_chunks(&[(CHUNK_VP8, b"\x50\x5a\x00\x9d\x01\x2a\xcd\x02\x00\x02")]);
        assert_eq!(webp.dimensions(), Some((717, 512)));
    }

    #[test]
    fn malformed_vp8_dimensions() {
        for data in [
            &b""[..],
            b"\x50\x5a\x00\x9d\x01\x2a",
            b"\x50\x5a\x00\x00\x00\x00\xcd\x02\x00\x02",
            b"\x51\x5a\x00\x9d\x01\x2a\xcd\x02\x00\x02",
            b"\x50\x5a\x00\x9d\x01\x2a\x00\x00\x00\x02",
            b"\x50\x5a\x00\x9d\x01\x2a\xcd\x02\x00\x00",
            b"\x50\x5a\x00\x9d\x01\x2a\x00\xc0\x00\x02",
        ] {
            let webp = from_chunks(&[(CHUNK_VP8, data)]);
            assert_eq!(webp.dimensions(), None);
        }
    }

    #[test]
    fn set_metadata_unknown_dimensions() {
        for mut webp in [
            from_chunks(&[]),
            from_chunks(&[(CHUNK_VP8, b"\x50\x5a\x00\x9d\x01\x2a")]),
            from_chunks(&[(CHUNK_VP8L, b"\x2f\x00")]),
        ] {
            webp.set_icc_profile(Some(Bytes::from_static(b"icc")));
            webp.set_exif(Some(Bytes::from_static(b"exif")));
            assert!(webp.has_chunk(CHUNK_ICCP) && webp.has_chunk(CHUNK_EXIF));
            assert!(!webp.has_chunk(CHUNK_VP8X));
        }
    }
}
