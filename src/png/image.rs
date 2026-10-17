use alloc::vec::Vec;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use miniz_oxide::{deflate::compress_to_vec_zlib, inflate::decompress_to_vec_zlib_with_limit};

use super::PngChunk;
use crate::{
    encoder::{EncodeAt, ImageEncoder},
    util::read_u8_array,
    Error, ImageEXIF, ImageICC, Result,
};

// the 8 byte signature
pub(crate) const SIGNATURE: &[u8] = &[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

// profiles decompressing to more than this are ignored, so that a small
// iCCP chunk can't allocate unbounded memory. libpng rejects profiles
// bigger than 8 MB by default, so real files aren't affected
const ICC_PROFILE_MAX_SIZE: usize = 16 * 1024 * 1024;

pub const CHUNK_ICCP: [u8; 4] = *b"iCCP";
pub const CHUNK_EXIF: [u8; 4] = *b"eXIf";
pub const CHUNK_IEND: [u8; 4] = *b"IEND";

/// The representation of a Png image
#[derive(Debug, Clone, PartialEq, Eq)]
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
        let signature: [u8; SIGNATURE.len()] = read_u8_array(&mut b)?;
        if signature != SIGNATURE {
            return Err(Error::WrongSignature);
        }

        let mut chunks = Vec::with_capacity(8);
        while !b.is_empty() {
            let chunk = PngChunk::from_bytes(&mut b)?;

            // Often PNG images found in the internet contain garbage after IEND chunk.
            // Most PNG parsers simply ignore everything after IEND chunk
            let is_end = chunk.kind() == CHUNK_IEND;
            chunks.push(chunk);

            if is_end {
                break;
            }
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
        let mut contents = self.chunk_by_type(CHUNK_ICCP)?.contents().clone();

        // skip nul-terminated profile name
        while contents.try_get_u8().ok()? != 0 {}

        // match on the compression method
        match contents.try_get_u8().ok()? {
            0 => decompress_to_vec_zlib_with_limit(&contents, ICC_PROFILE_MAX_SIZE)
                .ok()
                .map(Bytes::from),
            _ => None,
        }
    }

    fn set_icc_profile(&mut self, profile: Option<Bytes>) {
        self.remove_chunks_by_type(CHUNK_ICCP);

        if let Some(profile) = profile {
            let mut contents = BytesMut::with_capacity(profile.len());
            // profile name written as a C string
            contents.extend_from_slice(b"icc\0");
            // compression method
            contents.put_u8(0);
            // compressed profile
            let compressed = compress_to_vec_zlib(&profile, 10);
            contents.extend_from_slice(&compressed);

            let chunk = PngChunk::new(CHUNK_ICCP, contents.freeze());
            // right after the IHDR chunk
            let pos = self.chunks.len().min(1);
            self.chunks.insert(pos, chunk);
        }
    }
}

// https://www.w3.org/TR/png-3/#eXIf
impl ImageEXIF for Png {
    fn exif(&self) -> Option<Bytes> {
        Some(self.chunk_by_type(CHUNK_EXIF)?.contents().clone())
    }

    fn set_exif(&mut self, exif: Option<Bytes>) {
        self.remove_chunks_by_type(CHUNK_EXIF);

        if let Some(exif) = exif {
            let chunk = PngChunk::new(CHUNK_EXIF, exif);
            // right after the IHDR chunk, since it must come before the image data
            let pos = self.chunks.len().min(1);
            self.chunks.insert(pos, chunk);
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn icc_profile() {
        let mut png = Png {
            chunks: vec![PngChunk::new(*b"IHDR", Bytes::new())],
        };
        let profile = Bytes::from_static(b"profile");
        png.set_icc_profile(Some(profile.clone()));
        assert_eq!(png.icc_profile(), Some(profile));
    }

    #[test]
    fn icc_profile_size_limit() {
        let mut png = Png {
            chunks: vec![PngChunk::new(*b"IHDR", Bytes::new())],
        };

        png.set_icc_profile(Some(vec![0; ICC_PROFILE_MAX_SIZE].into()));
        let len = png.icc_profile().map(|profile| profile.len());
        assert_eq!(len, Some(ICC_PROFILE_MAX_SIZE));

        png.set_icc_profile(Some(vec![0; ICC_PROFILE_MAX_SIZE + 1].into()));
        assert!(png.icc_profile().is_none());
    }

    #[test]
    fn set_metadata_without_chunks() {
        let png = Png::from_bytes(Bytes::from_static(SIGNATURE)).unwrap();

        let mut with_icc = png.clone();
        let profile = Bytes::from_static(b"profile");
        with_icc.set_icc_profile(Some(profile.clone()));
        assert_eq!(with_icc.icc_profile(), Some(profile));

        let mut with_exif = png;
        let exif = Bytes::from_static(b"exif");
        with_exif.set_exif(Some(exif.clone()));
        assert_eq!(with_exif.exif(), Some(exif));
    }

    #[test]
    fn set_exif_position() {
        let mut png = Png {
            chunks: vec![
                PngChunk::new(*b"IHDR", Bytes::new()),
                PngChunk::new(*b"IDAT", Bytes::new()),
                PngChunk::new(CHUNK_IEND, Bytes::new()),
            ],
        };
        png.set_exif(Some(Bytes::from_static(b"exif")));

        let kinds: Vec<[u8; 4]> = png.chunks().iter().map(PngChunk::kind).collect();
        assert_eq!(kinds, [*b"IHDR", CHUNK_EXIF, *b"IDAT", CHUNK_IEND]);
    }

    #[test]
    fn malformed_icc_profile() {
        for contents in [&b""[..], b"icc", b"icc\0"] {
            let png = Png {
                chunks: vec![PngChunk::new(CHUNK_ICCP, Bytes::from_static(contents))],
            };
            assert_eq!(png.icc_profile(), None);
        }
    }
}
