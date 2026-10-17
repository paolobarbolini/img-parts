use alloc::vec::Vec;
use core::fmt;

use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::{
    encoder::{Encode, ImageEncoder},
    util::{read_checked, read_u8_array, split_to_checked},
    Error, Result,
};

// the 4 bytes signature
const SIGNATURE: &[u8] = b"RIFF";

// lists nested deeper than this are kept as data instead of being
// parsed, so that deeply nested lists can't overflow the stack
const MAX_LIST_DEPTH: usize = 32;

/// The representation of a RIFF chunk
#[derive(Clone, PartialEq)]
pub struct RiffChunk {
    id: [u8; 4],
    content: RiffContent,
}

/// The contents of a RIFF chunk
#[derive(Debug, Clone, PartialEq)]
pub enum RiffContent {
    List {
        kind: Option<[u8; 4]>,
        subchunks: Vec<RiffChunk>,
    },
    Data(Bytes),
}

#[allow(clippy::len_without_is_empty)]
impl RiffChunk {
    /// Construct a new RIFF chunk.
    #[inline]
    pub fn new(id: [u8; 4], content: RiffContent) -> RiffChunk {
        RiffChunk { id, content }
    }

    /// Create a new `RiffChunk` image from a Reader.
    ///
    /// Lists nested more than 32 levels deep aren't parsed,
    /// their content is kept as [`RiffContent::Data`].
    ///
    /// # Errors
    ///
    /// This method fails if the file signature doesn't match or one
    /// of the chunks is corrupted or truncated.
    #[inline]
    pub fn from_bytes(mut b: Bytes) -> Result<RiffChunk> {
        RiffChunk::from_bytes_impl(&mut b, true, 0)
    }

    pub(crate) fn from_bytes_impl(
        b: &mut Bytes,
        check_riff_id: bool,
        depth: usize,
    ) -> Result<RiffChunk> {
        let id: [u8; SIGNATURE.len()] = read_u8_array(b)?;
        if check_riff_id && id != SIGNATURE {
            return Err(Error::WrongSignature);
        }

        let content = RiffContent::from_bytes(b, id, depth)?;
        Ok(RiffChunk::new(id, content))
    }

    /// Get the id of this `RiffChunk`
    #[inline]
    pub fn id(&self) -> [u8; 4] {
        self.id
    }

    /// Get the content of this `RiffChunk`
    #[inline]
    pub fn content(&self) -> &RiffContent {
        &self.content
    }

    /// Get a mutable reference to the content of this `RiffChunk`
    #[inline]
    pub fn content_mut(&mut self) -> &mut RiffContent {
        &mut self.content
    }

    /// Get the total size of this `RiffChunk` once it is encoded.
    ///
    /// The size is the sum of:
    ///
    /// - The chunk id (4 bytes).
    /// - The size field (4 bytes).
    /// - The size of the content + a single padding byte if the size is odd.
    pub fn len(&self) -> u32 {
        let mut len = 4 + 4 + self.content.len();

        // RIFF chunks with an uneven number of bytes have an extra 0x00 padding byte
        len += len % 2;

        len
    }

    /// Returns an encoder for this `RiffChunk`
    #[inline]
    pub fn encoder(self) -> ImageEncoder<Self> {
        ImageEncoder::from(self)
    }
}

impl Encode for RiffChunk {
    fn encode(&self, pieces: &mut Vec<Bytes>) {
        let mut vec = BytesMut::with_capacity(8);
        vec.extend_from_slice(&self.id);
        vec.put_u32_le(self.content.len());
        pieces.push(vec.freeze());

        self.content.encode(pieces);
    }
}

#[allow(clippy::len_without_is_empty)]
impl RiffContent {
    fn from_bytes(b: &mut Bytes, id: [u8; 4], depth: usize) -> Result<RiffContent> {
        let len = read_checked(b, |b| b.get_u32_le())?;
        let mut content = split_to_checked(b, len as usize)?;

        if has_subchunks(id) && depth < MAX_LIST_DEPTH {
            let kind = if has_kind(id) {
                Some(read_u8_array(&mut content)?)
            } else {
                None
            };

            let mut subchunks = Vec::with_capacity(8);
            while !content.is_empty() {
                let subchunk = RiffChunk::from_bytes_impl(&mut content, false, depth + 1)?;
                subchunks.push(subchunk);
            }

            Ok(RiffContent::List { kind, subchunks })
        } else {
            // RIFF chunks with an uneven number of bytes have an extra 0x00 padding byte
            if len % 2 != 0 {
                read_checked(b, |b| b.get_u8())?;
            }

            Ok(RiffContent::Data(content))
        }
    }

    /// Get the total size of this `RiffContent` once it is encoded.
    ///
    /// If this `RiffContent` is a `List` the size is the sum of:
    ///
    /// - The kind (4 bytes) if this `List` has a kind.
    /// - The sum of the size of every `subchunk`.
    ///
    /// If this `RiffContent` is `Data` the size is the length of the data.
    pub fn len(&self) -> u32 {
        match self {
            RiffContent::List { kind, subchunks } => {
                let mut len = 0;

                if kind.is_some() {
                    len += 4;
                }

                len += subchunks.iter().map(|subchunk| subchunk.len()).sum::<u32>();
                len
            }
            RiffContent::Data(data) => data.len().try_into().unwrap(),
        }
    }

    /// Get `kind` and `subchunks` of this `RiffContent` if it is a `List`.
    ///
    /// Returns `None` if it is `Data`.
    pub fn list(&self) -> Option<(Option<[u8; 4]>, &Vec<RiffChunk>)> {
        match self {
            RiffContent::List { kind, subchunks } => Some((*kind, subchunks)),
            RiffContent::Data(_) => None,
        }
    }

    /// Get the `data` of this `RiffContent` if it is `Data`.
    ///
    /// Returns `None` if it is a `List`.
    pub fn data(&self) -> Option<&Bytes> {
        match self {
            RiffContent::List { .. } => None,
            RiffContent::Data(data) => Some(data),
        }
    }

    /// Returns an encoder for this `RiffContent`
    #[inline]
    pub fn encoder(self) -> ImageEncoder<Self> {
        ImageEncoder::from(self)
    }
}

impl Encode for RiffContent {
    fn encode(&self, pieces: &mut Vec<Bytes>) {
        match self {
            RiffContent::List { kind, subchunks } => {
                if let Some(kind) = kind {
                    pieces.push(Bytes::copy_from_slice(kind.as_ref()));
                }

                for chunk in subchunks {
                    chunk.encode(pieces);
                }
            }
            RiffContent::Data(data) => {
                pieces.push(data.clone());

                // RIFF chunks with an uneven number of bytes have an extra 0x00 padding byte
                if data.len() % 2 == 1 {
                    pieces.push(Bytes::from_static(&[0x00]));
                }
            }
        }
    }
}

impl fmt::Debug for RiffChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RiffChunk").field("id", &self.id).finish()
    }
}

fn has_subchunks(id: [u8; 4]) -> bool {
    matches!(&id, b"RIFF" | b"LIST" | b"seqt")
}

fn has_kind(id: [u8; 4]) -> bool {
    matches!(&id, b"RIFF" | b"LIST")
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::*;

    #[test]
    fn deeply_nested_lists() {
        // "seqt" lists nested 100000 levels deep
        let depth = 100_000;
        let mut buf = Vec::with_capacity(12 + 8 * depth);
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&(4 + 8 * depth as u32).to_le_bytes());
        buf.extend_from_slice(b"WEBP");
        for i in (0..depth).rev() {
            buf.extend_from_slice(b"seqt");
            buf.extend_from_slice(&(8 * i as u32).to_le_bytes());
        }
        let buf = Bytes::from(buf);

        let riff = RiffChunk::from_bytes(buf.clone()).unwrap();

        let mut chunk = &riff;
        let mut lists = 0;
        while let Some((_, subchunks)) = chunk.content().list() {
            chunk = &subchunks[0];
            lists += 1;
        }
        assert_eq!(lists, MAX_LIST_DEPTH);
        assert!(chunk.content().data().is_some());

        assert_eq!(riff.encoder().bytes(), buf);
    }
}
