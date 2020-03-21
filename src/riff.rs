use std::io::{Read, Result, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

#[derive(Debug, Clone, PartialEq)]
pub struct RiffChunk {
    id: [u8; 4],
    contents: Vec<u8>,
}

impl RiffChunk {
    pub fn new(id: [u8; 4], contents: Vec<u8>) -> RiffChunk {
        RiffChunk { id, contents }
    }

    pub fn read(r: &mut dyn Read) -> Result<RiffChunk> {
        let mut id: [u8; 4] = [0; 4];
        r.read_exact(&mut id)?;

        RiffChunk::read_skipping_id(r, id)
    }

    pub(crate) fn read_skipping_id(r: &mut dyn Read, id: [u8; 4]) -> Result<RiffChunk> {
        let len: u32 = r.read_u32::<LittleEndian>()?;

        let mut contents = Vec::with_capacity(len as usize);
        r.take(len as u64).read_to_end(&mut contents)?;

        if len % 2 != 0 {
            r.read_u8()?;
        }

        Ok(RiffChunk::new(id, contents))
    }

    pub fn id(&self) -> &[u8] {
        &self.id
    }

    pub fn contents(&self) -> &[u8] {
        self.contents.as_slice()
    }

    pub fn size(&self) -> usize {
        let mut len = 4 + 4 + self.contents.len();

        if self.contents.len() % 2 != 0 {
            len += 1;
        }

        len
    }

    pub fn write_to(&self, w: &mut dyn Write) -> Result<()> {
        w.write_all(&self.id)?;
        w.write_u32::<LittleEndian>(self.contents().len() as u32)?;
        w.write_all(&self.contents)?;

        if self.contents.len() % 2 != 0 {
            w.write_u8(0x00)?;
        }

        Ok(())
    }
}
