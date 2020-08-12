use bytes::{Buf, BufMut, Bytes, BytesMut};
use miniz_oxide::deflate::compress_to_vec_zlib;
use miniz_oxide::inflate::decompress_to_vec_zlib;

use super::PngChunk;
use crate::encoder::{EncodeAt, ImageEncoder};
use crate::util::read_u8_len8_array;
use crate::{Error, ImageEXIF, ImageICC, Result};

// the 8 byte signature
pub(crate) const SIGNATURE: &[u8] = &[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

pub const CHUNK_ICCP: [u8; 4] = [b'i', b'C', b'C', b'P'];
pub const CHUNK_EXIF: [u8; 4] = [b'e', b'X', b'I', b'f'];

/// The representation of a Png image
#[derive(Debug, Clone, PartialEq)]
pub struct Png {
    chunks: Vec<PngChunk>,
}

#[allow(clippy::len_without_is_empty)]
impl Png {
    /// Create a `Png` from `Bytes`
    ///
    /// # Errors
    ///
    /// This method fails if the file signature doesn't match or if
    /// it is corrupted or truncated.
    pub fn from_bytes(mut b: Bytes) -> Result<Png> {
        let signature: [u8; SIGNATURE.len()] = read_u8_len8_array(&mut b)?;
        if signature != SIGNATURE {
            return Err(Error::WrongSignature);
        }

        let mut chunks = Vec::with_capacity(8);
        while !b.is_empty() {
            let chunk = PngChunk::from_bytes(&mut b)?;
            chunks.push(chunk);
        }

        Ok(Png { chunks })
    }

    /// Get the chunks of this `Png`
    #[inline]
    pub fn chunks(&self) -> &Vec<PngChunk> {
        &self.chunks
    }

    /// Get a mutable reference to the chunks of this `Png`
    #[inline]
    pub fn chunks_mut(&mut self) -> &mut Vec<PngChunk> {
        &mut self.chunks
    }

    /// Get the first chunk with a type of `kind`
    pub fn chunk_by_type(&self, kind: [u8; 4]) -> Option<&PngChunk> {
        self.chunks.iter().find(|chunk| chunk.kind() == kind)
    }

    /// Get every chunk with a type of `kind`
    pub fn chunks_by_type(&self, kind: [u8; 4]) -> impl Iterator<Item = &PngChunk> {
        self.chunks.iter().filter(move |chunk| chunk.kind() == kind)
    }

    /// Remove every chunk with a type of `kind`
    pub fn remove_chunks_by_type(&mut self, kind: [u8; 4]) {
        self.chunks_mut().retain(|chunk| chunk.kind() != kind);
    }

    /// Get the total size of the `Png` once it is encoded.
    ///
    /// The size is the sum of:
    ///
    /// - The signature (8 bytes).
    /// - The size of every chunk.
    pub fn len(&self) -> usize {
        // signature (8 bytes) + length of every chunk
        8 + self.chunks.iter().map(|chunk| chunk.len()).sum::<usize>()
    }

    /// Create an [encoder][crate::ImageEncoder] for this `Png`
    #[inline]
    pub fn encoder(self) -> ImageEncoder<Self> {
        ImageEncoder::from(self)
    }
}

impl EncodeAt for Png {
    fn encode_at(&self, pos: &mut usize) -> Option<Bytes> {
        match pos {
            0 => Some(Bytes::from_static(SIGNATURE)),
            _ => {
                *pos -= 1;

                for chunk in &self.chunks {
                    if let Some(bytes) = chunk.encode_at(pos) {
                        return Some(bytes);
                    }
                }

                None
            }
        }
    }

    fn len(&self) -> usize {
        self.len()
    }
}

// http://www.libpng.org/pub/png/spec/1.2/PNG-Chunks.html#C.iCCP
impl ImageICC for Png {
    fn icc_profile(&self) -> Option<Bytes> {
        let mut contents = self
            .chunk_by_type(CHUNK_ICCP)
            .map(|chunk| chunk.contents().clone())?;

        // skip profile name and null separator
        while contents.get_u8() != 0 {}

        // check that the compression method is zlib
        if contents.get_u8() != 0 {
            return None;
        }

        decompress_to_vec_zlib(&contents).ok().map(Bytes::from)
    }

    fn set_icc_profile(&mut self, profile: Option<Bytes>) {
        self.remove_chunks_by_type(CHUNK_ICCP);

        if let Some(profile) = profile {
            let mut contents = BytesMut::with_capacity(profile.len());
            // profile name
            contents.extend_from_slice(b"icc");
            // null separator
            contents.put_u8(0);
            // compression method
            contents.put_u8(0);
            // compressed profile
            let compressed = compress_to_vec_zlib(&profile, 10);
            contents.extend_from_slice(&compressed);

            let chunk = PngChunk::new(CHUNK_ICCP, contents.freeze());
            self.chunks.insert(1, chunk);
        }
    }
}

// https://ftp-osl.osuosl.org/pub/libpng/documents/pngext-1.5.0.html#C.eXIf
impl ImageEXIF for Png {
    fn exif(&self) -> Option<Bytes> {
        self.chunk_by_type(CHUNK_EXIF)
            .map(|chunk| chunk.contents().clone())
    }

    fn set_exif(&mut self, exif: Option<Bytes>) {
        self.remove_chunks_by_type(CHUNK_EXIF);

        if let Some(exif) = exif {
            let chunk = PngChunk::new(CHUNK_EXIF, exif);
            self.chunks.insert(self.chunks.len() - 1, chunk);
        }
    }
}
