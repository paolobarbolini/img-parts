use std::convert::TryInto;
use std::fmt;
use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::{Error, Result};

#[derive(Clone, PartialEq)]
pub struct RiffChunk {
    id: [u8; 4],
    content: RiffContent,
}

#[derive(Clone, PartialEq)]
pub enum RiffContent {
    List {
        kind: Option<[u8; 4]>,
        subchunks: Vec<RiffChunk>,
    },
    Data(Vec<u8>),
}

#[allow(clippy::len_without_is_empty)]
impl RiffChunk {
    #[inline]
    pub fn new(id: [u8; 4], content: RiffContent) -> RiffChunk {
        RiffChunk { id, content }
    }

    #[inline]
    pub fn read(r: &mut dyn Read) -> Result<RiffChunk> {
        RiffChunk::read_with_limits(r, u32::max_value())
    }

    #[inline]
    pub fn read_with_limits(r: &mut dyn Read, limit: u32) -> Result<RiffChunk> {
        RiffChunk::read_with_limits_(r, limit, true)
    }

    pub(crate) fn read_with_limits_(
        r: &mut dyn Read,
        limit: u32,
        check_riff_id: bool,
    ) -> Result<RiffChunk> {
        let mut id: [u8; 4] = [0; 4];
        r.read_exact(&mut id)?;

        RiffChunk::read_contents(r, id, limit - 2, check_riff_id)
    }

    pub(crate) fn read_contents(
        r: &mut dyn Read,
        id: [u8; 4],
        limit: u32,
        check_riff_id: bool,
    ) -> Result<RiffChunk> {
        if check_riff_id && id != *b"RIFF" {
            return Err(Error::NoRiffHeader);
        }

        let content = RiffContent::read_with_limits(r, id, limit)?;
        Ok(RiffChunk::new(id, content))
    }

    #[inline]
    pub fn id(&self) -> [u8; 4] {
        self.id
    }

    #[inline]
    pub fn content(&self) -> &RiffContent {
        &self.content
    }

    #[inline]
    pub fn content_mut(&mut self) -> &mut RiffContent {
        &mut self.content
    }

    pub fn len(&self) -> u32 {
        // FourCC (32 bit) + Size (32 bit) + Content
        let mut len = 4 + 4 + self.content.size();

        if len % 2 != 0 {
            len += 1;
        }

        len
    }

    pub fn write_to(&self, w: &mut dyn Write) -> Result<()> {
        w.write_all(&self.id)?;
        w.write_u32::<LittleEndian>(self.content.size().try_into().unwrap())?;
        self.content.write_to(w)
    }
}

impl RiffContent {
    fn read_with_limits(r: &mut dyn Read, id: [u8; 4], limit: u32) -> Result<RiffContent> {
        let mut len = r.read_u32::<LittleEndian>()?;
        if len > limit {
            return Err(Error::LimitExcedeed);
        }

        if has_subchunks(id) {
            let kind = if has_kind(id) {
                len -= 4;

                let mut buf = [0u8; 4];
                r.read_exact(&mut buf)?;

                Some(buf)
            } else {
                None
            };

            let mut subchunks = Vec::new();
            while len > 0 {
                let subchunk = RiffChunk::read_with_limits_(r, len, false)?;
                len -= subchunk.len();
                subchunks.push(subchunk);
            }

            Ok(RiffContent::List { kind, subchunks })
        } else {
            let mut content = Vec::with_capacity(len as usize);
            r.take(len as u64).read_to_end(&mut content)?;

            if len % 2 != 0 {
                r.read_u8()?;
            }

            Ok(RiffContent::Data(content))
        }
    }

    pub fn size(&self) -> u32 {
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

    pub fn list(&self) -> Option<(&Option<[u8; 4]>, &Vec<RiffChunk>)> {
        match self {
            RiffContent::List {
                ref kind,
                ref subchunks,
            } => Some((kind, subchunks)),
            RiffContent::Data(_) => None,
        }
    }

    pub fn data(&self) -> Option<&Vec<u8>> {
        match self {
            RiffContent::List { .. } => None,
            RiffContent::Data(data) => Some(data),
        }
    }

    pub fn write_to(&self, w: &mut dyn Write) -> Result<()> {
        match self {
            RiffContent::List { kind, subchunks } => {
                if let Some(kind) = kind {
                    w.write_all(kind)?;
                }

                for chunk in subchunks {
                    chunk.write_to(w)?;
                }
            }
            RiffContent::Data(data) => {
                w.write_all(data)?;

                if data.len() % 2 != 0 {
                    w.write_u8(0x00)?;
                }
            }
        };

        Ok(())
    }
}

impl fmt::Debug for RiffChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RiffChunk").field("id", &self.id).finish()
    }
}

fn has_subchunks(id: [u8; 4]) -> bool {
    match &id {
        b"RIFF" | b"LIST" | b"seqt" => true,
        _ => false,
    }
}

fn has_kind(id: [u8; 4]) -> bool {
    match &id {
        b"RIFF" | b"LIST" => true,
        _ => false,
    }
}
