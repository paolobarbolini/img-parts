use std::io::{Read, Result};

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};

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
        4 + 4 + self.contents.len()
    }

    pub fn bytes(mut self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.len());

        bytes.extend(&self.id);

        let mut len: [u8; 4] = [0; 4];
        LittleEndian::write_u32(&mut len, self.contents.len() as u32);
        bytes.extend(&len);

        bytes.append(&mut self.contents);
        bytes
    }
}
