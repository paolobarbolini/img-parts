use alloc::vec::Vec;
use core::fmt;

use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::encoder::{EncodeAt, ImageEncoder};
use crate::util::{read_checked, read_u8_array, split_to_checked};
use crate::{Error, Result};

// the 4 bytes signature
const SIGNATURE: &[u8] = b"RIFF";

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
    /// # Errors
    ///
    /// This method fails if the file signature doesn't match or one
    /// of the chunks is corrupted or truncated.
    #[inline]
    pub fn from_bytes(mut b: Bytes) -> Result<RiffChunk> {
        RiffChunk::from_bytes_impl(&mut b, true)
    }

    pub(crate) fn from_bytes_impl(b: &mut Bytes, check_riff_id: bool) -> Result<RiffChunk> {
        let id: [u8; SIGNATURE.len()] = read_u8_array(b)?;
        if check_riff_id && id != SIGNATURE {
            return Err(Error::WrongSignature);
        }

        let content = RiffContent::from_bytes(b, id)?;
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

impl EncodeAt for RiffChunk {
    fn encode_at(&self, pos: &mut usize) -> Option<Bytes> {
        match pos {
            0 => {
                let mut vec = BytesMut::with_capacity(8);
                vec.extend_from_slice(&self.id);
                vec.put_u32_le(self.content.len());

                Some(vec.freeze())
            }
            _ => {
                *pos -= 1;
                self.content.encode_at(pos)
            }
        }
    }

    fn len(&self) -> usize {
        self.len() as usize
    }
}

#[allow(clippy::len_without_is_empty)]
impl RiffContent {
    fn from_bytes(b: &mut Bytes, id: [u8; 4]) -> Result<RiffContent> {
        let len = read_checked(b, |b| b.get_u32_le())?;
        let mut content = split_to_checked(b, len as usize)?;

        if has_subchunks(id) {
            let kind = if has_kind(id) {
                Some(read_u8_array(&mut content)?)
            } else {
                None
            };

            let mut subchunks = Vec::with_capacity(8);
            while !content.is_empty() {
                let subchunk = RiffChunk::from_bytes_impl(&mut content, false)?;
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

impl EncodeAt for RiffContent {
    fn encode_at(&self, pos: &mut usize) -> Option<Bytes> {
        match self {
            RiffContent::List { kind, subchunks } => {
                if let Some(kind) = kind {
                    if *pos == 0 {
                        return Some(Bytes::copy_from_slice(kind.as_ref()));
                    }

                    *pos -= 1;
                };

                for chunk in subchunks {
                    if let Some(bytes) = chunk.encode_at(pos) {
                        return Some(bytes);
                    }
                }

                None
            }
            RiffContent::Data(data) => match pos {
                0 => Some(data.clone()),
                1 if data.len() % 2 == 1 => Some(Bytes::from_static(&[0x00])),
                _ => {
                    *pos -= 1 + data.len() % 2;
                    None
                }
            },
        }
    }

    fn len(&self) -> usize {
        self.len() as usize
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
