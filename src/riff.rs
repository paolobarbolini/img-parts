use std::io::{Read, Result};

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};

#[derive(Debug, Clone, PartialEq)]
pub struct RiffChunk {
    id: [u8; 4],
    contents: Vec<u8>,
}

#[allow(clippy::len_without_is_empty)]
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

        Ok(RiffChunk::new(id, contents))
    }

    pub fn id(&self) -> &[u8] {
        &self.id
    }

    pub fn contents(&self) -> &[u8] {
        self.contents.as_slice()
    }

    pub fn len(&self) -> usize {
        let mut len = 4 + 4 + self.contents.len();

        if self.contents.len() % 2 != 0 {
            len += 1;
        }

        len
    }

    pub fn bytes(mut self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.len());

        bytes.extend(&self.id);

        let mut len: [u8; 4] = [0; 4];
        LittleEndian::write_u32(&mut len, self.contents.len() as u32);
        bytes.extend(&len);

        let final_bit = self.contents.len() % 2 != 0;

        bytes.append(&mut self.contents);

        if final_bit {
            bytes.push(0x00);
        }

        bytes
    }
}
