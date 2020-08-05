use std::io;

use bytes::{Buf, Bytes};

use super::PngChunk;
use crate::encoder::{EncodeAt, ImageEncoder};
use crate::{Error, Result};

// the 8 byte signature
const SIGNATURE: &[u8] = &[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

/// The representation of a Png image.
#[derive(Debug, Clone, PartialEq)]
pub struct Png {
    chunks: Vec<PngChunk>,
}

#[allow(clippy::len_without_is_empty)]
impl Png {
    /// Create a new `Png` image from a Reader.
    ///
    /// # Errors
    ///
    /// This method fails if reading fails or if the file signature is invalid.
    pub fn from_bytes(mut b: Bytes) -> Result<Png> {
        let mut signature = [0; SIGNATURE.len()];
        b.copy_to_slice(&mut signature);
        if signature != SIGNATURE {
            return Err(Error::WrongSignature);
        }

        let mut chunks = Vec::new();
        loop {
            let chunk = match PngChunk::from_bytes(&mut b) {
                Ok(chunk) => chunk,
                Err(Error::Io(e)) if e.kind() == io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(e) => return Err(e),
            };
            chunks.push(chunk);
        }

        Ok(Png { chunks })
    }

    /// Get the chunks of this `Png`.
    #[inline]
    pub fn chunks(&self) -> &Vec<PngChunk> {
        &self.chunks
    }

    /// Get a mutable reference to the chunks of this `Png`.
    #[inline]
    pub fn chunks_mut(&mut self) -> &mut Vec<PngChunk> {
        &mut self.chunks
    }

    /// Get the first chunk with a marker of `marker`
    pub fn chunk_by_type(&self, kind: [u8; 4]) -> Option<&PngChunk> {
        self.chunks.iter().find(|chunk| chunk.kind() == kind)
    }

    /// Get every chunk with a marker of `marker`
    pub fn chunks_by_type(&self, kind: [u8; 4]) -> impl Iterator<Item = &PngChunk> {
        self.chunks.iter().filter(move |chunk| chunk.kind() == kind)
    }

    /// Remove every chunk with an id of `id`
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

    /// Returns an encoder for this `Png`
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
}
