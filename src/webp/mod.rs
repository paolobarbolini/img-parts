use alloc::vec::Vec;

use bytes::{BufMut, Bytes, BytesMut};

use crate::encoder::ImageEncoder;
use crate::riff::{RiffChunk, RiffContent};
use crate::util::{u24_from_le_bytes, u24_to_le_bytes};
use crate::vp8::size_from_vp8_header;
use crate::vp8::VP8Kind;
use crate::{Error, ImageEXIF, ImageICC, Result, EXIF_DATA_PREFIX};
use flags::WebPFlags;

mod flags;

pub const CHUNK_ALPH: [u8; 4] = [b'A', b'L', b'P', b'H'];
pub const CHUNK_ANIM: [u8; 4] = [b'A', b'N', b'I', b'M'];
pub const CHUNK_ANMF: [u8; 4] = [b'A', b'N', b'M', b'F'];
pub const CHUNK_EXIF: [u8; 4] = [b'E', b'X', b'I', b'F'];
pub const CHUNK_ICCP: [u8; 4] = [b'I', b'C', b'C', b'P'];
pub const CHUNK_VP8: [u8; 4] = [b'V', b'P', b'8', b' '];
pub const CHUNK_VP8L: [u8; 4] = [b'V', b'P', b'8', b'L'];
pub const CHUNK_VP8X: [u8; 4] = [b'V', b'P', b'8', b'X'];
pub const CHUNK_XMP: [u8; 4] = [b'X', b'M', b'P', b' '];

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
    /// Construct a new `WebP` image from a [`RiffChunk`][crate::riff::RiffChunk].
    ///
    /// # Errors
    ///
    /// This method returns a [`Error::WrongSignature`][crate::Error::WrongSignature]
    /// if the content of the [`RiffChunk`][crate::riff::RiffChunk] isn't a `List` or
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

            let (width, height) = self.dimensions().unwrap();

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
                let (width, height) = size_from_vp8_header(data);
                return Some((width as u32, height as u32));
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
        let data = self.chunk_by_id(CHUNK_EXIF)?.content().data()?;

        if data.starts_with(EXIF_DATA_PREFIX) {
            Some(data.slice(EXIF_DATA_PREFIX.len()..))
        } else {
            None
        }
    }

    fn set_exif(&mut self, exif: Option<Bytes>) {
        self.remove_chunks_by_id(CHUNK_EXIF);

        if let Some(exif) = exif {
            let mut contents = BytesMut::with_capacity(6 + exif.len());
            contents.put(EXIF_DATA_PREFIX);
            contents.put(exif);

            let chunk = RiffChunk::new(CHUNK_EXIF, RiffContent::Data(contents.freeze()));
            self.chunks_mut().push(chunk);
        }

        self.convert_into_infered_kind();
    }
}
