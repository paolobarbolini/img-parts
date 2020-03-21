use std::convert::TryInto;
use std::io::{self, Read, Write};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use super::entropy::{read_entropy, write_entropy};
use super::markers::{has_entropy, has_length};
use crate::Result;

pub struct JpegSegment {
    marker: u8,
    contents: Vec<u8>,
    entropy_data: Option<Vec<u8>>,
}

impl JpegSegment {
    #[inline]
    pub fn new(marker: u8) -> JpegSegment {
        JpegSegment {
            marker,
            contents: Vec::new(),
            entropy_data: None,
        }
    }

    #[inline]
    pub fn new_with_contents(marker: u8, contents: Vec<u8>) -> JpegSegment {
        JpegSegment {
            marker,
            contents,
            entropy_data: None,
        }
    }

    #[inline]
    pub fn new_with_entropy(marker: u8, contents: Vec<u8>, entropy: Vec<u8>) -> JpegSegment {
        JpegSegment {
            marker,
            contents,
            entropy_data: Some(entropy),
        }
    }

    pub fn read(marker: u8, r: &mut dyn Read) -> Result<JpegSegment> {
        let size = r.read_u16::<BigEndian>()? - 2;

        let mut contents = Vec::with_capacity(size as usize);
        r.take(size as u64).read_to_end(&mut contents)?;

        if !has_entropy(marker) {
            Ok(JpegSegment::new_with_contents(marker, contents))
        } else {
            let entropy = read_entropy(r)?;
            Ok(JpegSegment::new_with_entropy(marker, contents, entropy))
        }
    }

    pub fn size(&self) -> usize {
        if has_length(self.marker) {
            // 2 bytes (marker) + 2 bytes (length) + length of the content
            2 + 2 + self.contents.len()
        } else {
            // 2 bytes (marker) + length of the content
            2 + self.contents.len()
        }
    }

    #[inline]
    pub fn marker(&self) -> u8 {
        self.marker
    }

    #[inline]
    pub fn contents(&self) -> &[u8] {
        self.contents.as_slice()
    }

    #[inline]
    pub fn has_entropy(&self) -> bool {
        self.entropy_data.is_some()
    }

    pub fn write_to(&self, w: &mut dyn Write) -> io::Result<()> {
        w.write_u16::<BigEndian>((self.size() - 2).try_into().unwrap())?;
        w.write_all(&self.contents)?;

        if let Some(entropy) = &self.entropy_data {
            write_entropy(entropy, w)?;
        }

        Ok(())
    }
}
