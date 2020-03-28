use std::io::{Read, Write};

use byteorder::{ByteOrder, LittleEndian};

use crate::riff::{RiffChunk, RiffContent};
use crate::vp8::decode_size_vp8_from_header;
use crate::vp8::VP8Kind;
use crate::{Error, ImageICC, Result};

pub const CHUNK_ALPH: [u8; 4] = [b'A', b'L', b'P', b'H'];
pub const CHUNK_ANIM: [u8; 4] = [b'A', b'N', b'I', b'M'];
pub const CHUNK_ANMF: [u8; 4] = [b'A', b'N', b'M', b'F'];
pub const CHUNK_EXIF: [u8; 4] = [b'E', b'X', b'I', b'F'];
pub const CHUNK_ICCP: [u8; 4] = [b'I', b'C', b'C', b'P'];
pub const CHUNK_VP8: [u8; 4] = [b'V', b'P', b'8', b' '];
pub const CHUNK_VP8L: [u8; 4] = [b'V', b'P', b'8', b'L'];
pub const CHUNK_VP8X: [u8; 4] = [b'V', b'P', b'8', b'X'];
pub const CHUNK_XMP: [u8; 4] = [b'X', b'M', b'P', b' '];

#[derive(Debug, Clone, PartialEq)]
pub struct WebP {
    riff: RiffChunk,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct WebPFlags([u8; 4]);

#[allow(clippy::len_without_is_empty)]
impl WebP {
    pub fn read(r: &mut dyn Read) -> Result<WebP> {
        WebP::read_with_limits(r, u32::max_value())
    }

    pub fn read_with_limits(r: &mut dyn Read, limit: u32) -> Result<WebP> {
        let riff = RiffChunk::read_with_limits(r, limit)?;

        match riff.content().list() {
            Some((kind, _)) => {
                if kind == &Some(*b"WEBP") {
                    Ok(WebP { riff })
                } else {
                    Err(Error::NoWebpCC)
                }
            }
            None => Err(Error::NoWebpCC),
        }
    }

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
            let mut content = Vec::with_capacity(10);
            content.extend(&flags.0);

            let mut buf = [0u8; 3];
            LittleEndian::write_u24(&mut buf, width - 1);
            content.extend(&buf);
            LittleEndian::write_u24(&mut buf, height - 1);
            content.extend(&buf);

            let chunk = RiffChunk::new(CHUNK_VP8X, RiffContent::Data(content));
            self.chunks_mut().insert(pos, chunk);
        }
    }

    pub fn dimensions(&self) -> Option<(u32, u32)> {
        if let Ok(vp8x) = self.chunk_by_id(CHUNK_VP8X) {
            if let Some(data) = vp8x.content().data() {
                if let Some(range) = data.get(2..8) {
                    let width = LittleEndian::read_u24(&range[0..3]) + 1;
                    let height = LittleEndian::read_u24(&range[3..6]) + 1;
                    return Some((width, height));
                }
            }
        }

        if let Ok(vp8) = self.chunk_by_id(CHUNK_VP8) {
            let (width, height) = decode_size_vp8_from_header(vp8.content().data()?);
            return Some((width as u32, height as u32));
        }

        None
    }

    pub fn chunks(&self) -> &Vec<RiffChunk> {
        match self.riff.content() {
            RiffContent::List { subchunks, .. } => subchunks,
            _ => unreachable!(),
        }
    }

    pub fn chunks_mut(&mut self) -> &mut Vec<RiffChunk> {
        match self.riff.content_mut() {
            RiffContent::List {
                ref mut subchunks, ..
            } => subchunks,
            _ => unreachable!(),
        }
    }

    pub fn has_chunk(&self, id: [u8; 4]) -> bool {
        self.chunk_by_id(id).is_ok()
    }

    pub fn chunk_by_id(&self, id: [u8; 4]) -> Result<&RiffChunk> {
        self.chunks()
            .iter()
            .find(|chunk| chunk.id() == id)
            .ok_or_else(|| Error::NoChunk(id))
    }

    pub fn chunks_by_id(&self, id: [u8; 4]) -> Vec<&RiffChunk> {
        self.chunks()
            .iter()
            .filter(|chunk| chunk.id() == id)
            .collect()
    }

    pub fn remove_chunks_by_id(&mut self, id: [u8; 4]) {
        self.chunks_mut().retain(|chunk| chunk.id() != id);
    }

    #[inline]
    pub fn len(&self) -> u32 {
        self.riff.len()
    }

    #[inline]
    pub fn write_to(&self, w: &mut dyn Write) -> Result<()> {
        self.riff.write_to(w)
    }
}

impl ImageICC for WebP {
    fn icc_profile(&self) -> Option<Vec<u8>> {
        self.chunk_by_id(CHUNK_ICCP).ok()?.content().data().cloned()
    }

    fn set_icc_profile(&mut self, profile: Option<Vec<u8>>) {
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

impl WebPFlags {
    pub fn from_webp(webp: &WebP) -> WebPFlags {
        let mut flags = WebPFlags::default();
        if webp.has_chunk(CHUNK_ICCP) {
            flags.0[0] |= 0b0010_0000;
        }
        if webp.has_chunk(CHUNK_EXIF) {
            flags.0[0] |= 0b0000_1000;
        }
        flags
    }
}

impl Default for WebPFlags {
    fn default() -> Self {
        Self([0x00, 0x00, 0x00, 0x00])
    }
}
