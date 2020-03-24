use std::convert::{TryFrom, TryInto};
use std::io::{self, Read, Write};

use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};

use crate::riff::RiffChunk;
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
pub const CHUNK_XMP: [u8; 4] = [b'X', b'M', b'P', b' '];

#[derive(Debug, Clone, PartialEq)]
pub struct WebP {
    kind: VP8Kind,
    flags: WebPFlags,
    vp8x_canvas: Option<(u32, u32)>,
    chunks: Vec<RiffChunk>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct WebPFlags([u8; 4]);

impl WebP {
    pub fn read(r: &mut dyn Read) -> Result<WebP> {
        let mut buf = [0u8; 16];
        r.read_exact(&mut buf[0..16])?;

        if &buf[0..4] != b"RIFF" {
            return Err(Error::NoRiffHeader);
        }

        let len = LittleEndian::read_u32(&buf[4..8]) + 8;

        if &buf[8..12] != b"WEBP" {
            return Err(Error::NoWebpCC);
        }

        let kind = VP8Kind::from_bytes(&buf[12..16])
            .ok_or_else(|| Error::InvalidFormat(buf[12..16].try_into().unwrap()))?;
        match kind {
            VP8Kind::VP8 => {
                let id = buf[12..16].try_into().unwrap();
                let chunk = RiffChunk::read_skipping_id(r, id)?;

                Ok(WebP {
                    kind,
                    flags: WebPFlags::default(),
                    vp8x_canvas: None,
                    chunks: vec![chunk],
                })
            }
            VP8Kind::VP8L => unimplemented!(),
            VP8Kind::VP8X => {
                // size: 32 bits, flags: 32 bits, canvas width: 24 bits, canvas height: 24 bits
                r.read_exact(&mut buf[0..14])?;

                let flags = WebPFlags(buf[4..8].try_into().unwrap());
                let width = LittleEndian::read_u24(&buf[8..11]) + 1;
                let height = LittleEndian::read_u24(&buf[11..14]) + 1;

                let mut chunks = Vec::new();
                let mut read = 15;
                while (read + 16) < len {
                    let chunk = RiffChunk::read(r)?;
                    read += chunk.size() as u32;
                    chunks.push(chunk);
                }

                Ok(WebP {
                    kind,
                    flags,
                    vp8x_canvas: Some((width, height)),
                    chunks,
                })
            }
        }
    }

    #[inline]
    pub fn kind(&self) -> &VP8Kind {
        &self.kind
    }

    #[inline]
    pub fn dimensions(&self) -> Option<(u32, u32)> {
        self.vp8x_canvas
    }

    #[inline]
    pub fn chunks(&self) -> &[RiffChunk] {
        self.chunks.as_slice()
    }

    #[inline]
    pub fn chunks_mut(&mut self) -> &mut Vec<RiffChunk> {
        &mut self.chunks
    }

    #[inline]
    pub fn chunk_by_id(&self, id: [u8; 4]) -> Option<&RiffChunk> {
        self.chunks.iter().find(|chunk| chunk.id() == id)
    }

    #[inline]
    pub fn chunks_by_id(&self, id: [u8; 4]) -> Vec<&RiffChunk> {
        self.chunks
            .iter()
            .filter(|chunk| chunk.id() == id)
            .collect()
    }

    #[inline]
    pub fn remove_chunks_by_id(&mut self, id: [u8; 4]) {
        self.chunks.retain(|chunk| chunk.id() != id);
    }

    #[inline]
    pub fn size(&self) -> usize {
        self.size_with_kind(self.kind)
    }

    fn size_with_kind(&self, kind: VP8Kind) -> usize {
        // 12 bytes (header) + 4 bytes (kind)
        let mut len = 12 + 4;

        match kind {
            VP8Kind::VP8 => len += self.chunk_by_id(CHUNK_VP8).unwrap().size(),
            VP8Kind::VP8L => unimplemented!(),
            VP8Kind::VP8X => {
                // 4 bytes (Chunk Size) + 4 bytes (Flags) + 6 bytes (Canvas size)
                len += 4 + 4 + 6;

                // Sum of the length of every chunk
                len += self.chunks.iter().map(|chunk| chunk.size()).sum::<usize>();
            }
        };

        len
    }

    fn suggested_kind(&self) -> VP8Kind {
        let has_non_vp8_chunks = self
            .chunks
            .iter()
            .any(|chunk| VP8Kind::from_bytes(&chunk.id()).is_some());

        if has_non_vp8_chunks {
            VP8Kind::VP8X
        } else {
            VP8Kind::VP8
        }
    }

    pub fn write_to(&self, w: &mut dyn Write) -> io::Result<()> {
        let kind = self.suggested_kind();
        let len = self.size_with_kind(kind);

        // WebP file header
        w.write_all(b"RIFF")?;
        w.write_u32::<LittleEndian>(u32::try_from(len).unwrap() - 8)?;
        w.write_all(b"WEBP")?;

        match kind {
            VP8Kind::VP8 => {
                let vp8 = self.chunk_by_id(CHUNK_VP8).unwrap();
                vp8.write_to(w)?;
            }
            VP8Kind::VP8L => unimplemented!(),
            VP8Kind::VP8X => {
                // ChunkHeader
                w.write_all(&kind.to_bytes())?;

                // Chunk Size
                w.write_u32::<LittleEndian>(10)?;

                // Flags: 32 bits
                w.write_all(&self.flags.0)?;

                // Canvas: 24 bit + 24 bit
                if let Some((width, height)) = self.vp8x_canvas {
                    w.write_u24::<LittleEndian>(width - 1)?;
                    w.write_u24::<LittleEndian>(height - 1)?;
                } else {
                    let vp8 = self.chunk_by_id(CHUNK_VP8).unwrap();
                    let (width, height) = decode_size_vp8_from_header(vp8.contents());
                    w.write_u24::<LittleEndian>((width - 1).into())?;
                    w.write_u24::<LittleEndian>((height - 1).into())?;
                }

                for chunk in &self.chunks {
                    chunk.write_to(w)?;
                }
            }
        };

        Ok(())
    }
}

impl ImageICC for WebP {
    fn icc_profile(&self) -> Option<Vec<u8>> {
        self.chunk_by_id(CHUNK_ICCP)
            .map(|chunk| chunk.contents().to_vec())
    }
}

impl Default for WebPFlags {
    fn default() -> Self {
        Self([0x28, 0x00, 0x00, 0x00])
    }
}
