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

    /// Create a `WebP` from `Bytes`.
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
        // only the extended file format can have chunks other than the image data
        let extended = self
            .chunks()
            .iter()
            .any(|chunk| !matches!(chunk.id(), CHUNK_VP8 | CHUNK_VP8L | CHUNK_VP8X));

        if extended {
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
                let flags = WebPFlags::from_webp(self);
                let vp8x = self
                    .chunks_mut()
                    .iter_mut()
                    .find(|chunk| chunk.id() == CHUNK_VP8X);
                if let Some(RiffContent::Data(data)) = vp8x.map(RiffChunk::content_mut) {
                    if !data.is_empty() {
                        let mut content = BytesMut::from(&data[..]);
                        content[0] = flags.merge(content[0]);
                        *data = content.freeze();
                    }
                }
            }
        } else if correct_kind == VP8Kind::VP8 {
            self.remove_chunks_by_id(CHUNK_VP8X);
        } else if correct_kind == VP8Kind::VP8X {
            // TODO VP8L

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

            // the VP8X chunk must be the first chunk
            let chunk = RiffChunk::new(CHUNK_VP8X, RiffContent::Data(content.freeze()));
            self.chunks_mut().insert(0, chunk);
        }
    }

    /// Get the width and height of this `WebP`.
    ///
    /// If this `WebP` has a `VP8X` chunk the dimensions are the canvas size.
    ///
    /// Otherwise the dimensions are read from the VP8 or VP8L bitstream header.
    ///
    /// Returns `None` if the dimensions can't be read.
    pub fn dimensions(&self) -> Option<(u32, u32)> {
        if let Some(vp8x) = self.chunk_by_id(CHUNK_VP8X) {
            if let Some(data) = vp8x.content().data() {
                // the canvas size comes after 1 byte of flags and 3 reserved bytes
                if let Some(range) = data.get(4..10) {
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
            // the ICCP chunk goes right after the VP8X chunk,
            // which gets added in front of it if it's missing
            let pos = self
                .chunks()
                .iter()
                .position(|chunk| chunk.id() == CHUNK_VP8X)
                .map_or(0, |pos| pos + 1);

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
    use alloc::vec::Vec;

    use super::*;

    fn ids(webp: &WebP) -> Vec<[u8; 4]> {
        webp.chunks().iter().map(RiffChunk::id).collect()
    }

    fn from_chunks(chunks: &[([u8; 4], &[u8])]) -> WebP {
        let subchunks = chunks
            .iter()
            .map(|&(id, data)| RiffChunk::new(id, RiffContent::Data(Bytes::copy_from_slice(data))))
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
    fn vp8x_dimensions() {
        let webp = from_chunks(&[
            (CHUNK_VP8X, b"\x20\x00\x00\x00\xcc\x02\x00\xff\x01\x00"),
            (CHUNK_VP8, b"\x50\x5a\x00\x9d\x01\x2a\x01\x00\x01\x00"),
        ]);
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

    #[test]
    fn create_vp8x_first() {
        let vp8 = b"\x50\x5a\x00\x9d\x01\x2a\x01\x00\x01\x00";
        let mut webp = from_chunks(&[(CHUNK_VP8, vp8), (CHUNK_ICCP, b"icc")]);
        webp.set_exif(Some(Bytes::from_static(b"exif")));
        assert_eq!(ids(&webp), [CHUNK_VP8X, CHUNK_VP8, CHUNK_ICCP, CHUNK_EXIF]);
    }

    #[test]
    fn set_icc_profile_position() {
        let icc = Bytes::from_static(b"icc");
        let vp8 = b"\x50\x5a\x00\x9d\x01\x2a\x01\x00\x01\x00";

        let mut webp = from_chunks(&[(CHUNK_VP8, vp8)]);
        webp.set_icc_profile(Some(icc.clone()));
        assert_eq!(ids(&webp), [CHUNK_VP8X, CHUNK_ICCP, CHUNK_VP8]);

        let lossless = [0x20, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut webp = from_chunks(&[
            (CHUNK_VP8X, &lossless),
            (CHUNK_ICCP, b"old"),
            (CHUNK_VP8L, b"\x2f\x00\x00\x00\x00"),
        ]);
        webp.set_icc_profile(Some(icc.clone()));
        assert_eq!(ids(&webp), [CHUNK_VP8X, CHUNK_ICCP, CHUNK_VP8L]);

        let alpha = [0x10, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut webp = from_chunks(&[(CHUNK_VP8X, &alpha), (CHUNK_ALPH, b""), (CHUNK_VP8, vp8)]);
        webp.set_icc_profile(Some(icc.clone()));
        assert_eq!(ids(&webp), [CHUNK_VP8X, CHUNK_ICCP, CHUNK_ALPH, CHUNK_VP8]);

        let animation = [0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut webp = from_chunks(&[
            (CHUNK_VP8X, &animation),
            (CHUNK_ANIM, b""),
            (CHUNK_ANMF, b""),
        ]);
        webp.set_icc_profile(Some(icc));
        assert_eq!(ids(&webp), [CHUNK_VP8X, CHUNK_ICCP, CHUNK_ANIM, CHUNK_ANMF]);
    }

    #[test]
    fn create_vp8x_for_extended_features() {
        let vp8 = b"\x50\x5a\x00\x9d\x01\x2a\x01\x00\x01\x00";
        for (id, flags) in [
            (CHUNK_ALPH, 0x10),
            (CHUNK_XMP, 0x04),
            (CHUNK_ANIM, 0x02),
            (*b"ABCD", 0),
        ] {
            let mut webp = from_chunks(&[(CHUNK_VP8, vp8), (id, b"")]);
            webp.set_exif(None);
            assert_eq!(ids(&webp), [CHUNK_VP8X, CHUNK_VP8, id]);

            let vp8x = [flags, 0, 0, 0, 0, 0, 0, 0, 0, 0];
            let expected = RiffContent::Data(Bytes::copy_from_slice(&vp8x));
            assert_eq!(webp.chunk_by_id(CHUNK_VP8X).unwrap().content(), &expected);
        }
    }

    #[test]
    fn keep_vp8x_for_extended_features() {
        let vp8 = b"\x50\x5a\x00\x9d\x01\x2a\x01\x00\x01\x00";
        let vp8x = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        let mut webp = from_chunks(&[(CHUNK_VP8X, &vp8x), (CHUNK_ALPH, b""), (CHUNK_VP8, vp8)]);
        webp.set_exif(None);
        assert_eq!(ids(&webp), [CHUNK_VP8X, CHUNK_ALPH, CHUNK_VP8]);

        let mut webp = from_chunks(&[(CHUNK_VP8X, &vp8x), (CHUNK_ANIM, b""), (CHUNK_ANMF, b"")]);
        webp.set_icc_profile(None);
        assert_eq!(ids(&webp), [CHUNK_VP8X, CHUNK_ANIM, CHUNK_ANMF]);

        let mut webp = from_chunks(&[(CHUNK_VP8X, &vp8x), (CHUNK_VP8, vp8), (CHUNK_XMP, b"")]);
        webp.set_exif(None);
        assert_eq!(ids(&webp), [CHUNK_VP8X, CHUNK_VP8, CHUNK_XMP]);

        let mut webp = from_chunks(&[(CHUNK_VP8X, &vp8x), (*b"ABCD", b""), (CHUNK_VP8, vp8)]);
        webp.set_exif(None);
        assert_eq!(ids(&webp), [CHUNK_VP8X, *b"ABCD", CHUNK_VP8]);

        let mut webp = from_chunks(&[(CHUNK_VP8X, &vp8x), (CHUNK_ICCP, b""), (CHUNK_VP8, vp8)]);
        webp.set_icc_profile(None);
        assert_eq!(ids(&webp), [CHUNK_VP8]);
    }

    #[test]
    fn update_vp8x_flags() {
        let vp8 = b"\x50\x5a\x00\x9d\x01\x2a\x01\x00\x01\x00";
        let vp8x = |webp: &WebP| webp.chunk_by_id(CHUNK_VP8X).unwrap().content().clone();

        let mut webp = from_chunks(&[
            (CHUNK_VP8X, b"\x08\x00\x00\x00\xcc\x02\x00\xff\x01\x00"),
            (CHUNK_VP8, vp8),
            (CHUNK_EXIF, b"exif"),
        ]);
        webp.set_icc_profile(Some(Bytes::from_static(b"icc")));
        let expected = Bytes::from_static(b"\x28\x00\x00\x00\xcc\x02\x00\xff\x01\x00");
        assert_eq!(vp8x(&webp), RiffContent::Data(expected));

        let mut webp = from_chunks(&[
            (CHUNK_VP8X, b"\x30\x00\x00\x00\xcc\x02\x00\xff\x01\x00"),
            (CHUNK_ICCP, b"icc"),
            (CHUNK_ALPH, b""),
            (CHUNK_VP8, vp8),
        ]);
        webp.set_icc_profile(None);
        webp.set_exif(Some(Bytes::from_static(b"exif")));
        let expected = Bytes::from_static(b"\x18\x00\x00\x00\xcc\x02\x00\xff\x01\x00");
        assert_eq!(vp8x(&webp), RiffContent::Data(expected));

        let mut webp = from_chunks(&[
            (CHUNK_VP8X, b"\x28\x00\x00\x00\xcc\x02\x00\xff\x01\x00"),
            (CHUNK_ICCP, b"icc"),
            (CHUNK_VP8, vp8),
            (CHUNK_EXIF, b"exif"),
        ]);
        webp.set_exif(None);
        let expected = Bytes::from_static(b"\x20\x00\x00\x00\xcc\x02\x00\xff\x01\x00");
        assert_eq!(vp8x(&webp), RiffContent::Data(expected));

        // only the ICC and EXIF flags are updated
        let mut webp = from_chunks(&[
            (CHUNK_VP8X, b"\x00\x00\x00\x00\xcc\x02\x00\xff\x01\x00"),
            (CHUNK_VP8, vp8),
            (CHUNK_XMP, b"xmp"),
        ]);
        webp.set_icc_profile(Some(Bytes::from_static(b"icc")));
        let expected = Bytes::from_static(b"\x20\x00\x00\x00\xcc\x02\x00\xff\x01\x00");
        assert_eq!(vp8x(&webp), RiffContent::Data(expected));

        // a malformed empty VP8X chunk is left as is
        let mut webp = from_chunks(&[(CHUNK_VP8X, b""), (CHUNK_VP8, vp8), (CHUNK_EXIF, b"exif")]);
        webp.set_icc_profile(Some(Bytes::from_static(b"icc")));
        assert_eq!(vp8x(&webp), RiffContent::Data(Bytes::new()));
    }

    #[test]
    fn create_vp8x_for_lossless_alpha() {
        for (vp8l, flags) in [
            (b"\x2f\x00\x00\x00\x10", 0x18),
            (b"\x2f\x00\x00\x00\x00", 0x08),
        ] {
            let mut webp = from_chunks(&[(CHUNK_VP8L, vp8l)]);
            webp.set_exif(Some(Bytes::from_static(b"exif")));

            let vp8x = [flags, 0, 0, 0, 0, 0, 0, 0, 0, 0];
            let expected = RiffContent::Data(Bytes::copy_from_slice(&vp8x));
            assert_eq!(webp.chunk_by_id(CHUNK_VP8X).unwrap().content(), &expected);
        }
    }
}
