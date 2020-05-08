use std::io::{Read, Result, Write};

use byteorder::{ReadBytesExt, WriteBytesExt};

use super::markers;

#[derive(Clone, PartialEq)]
pub struct Entropy {
    raw: Vec<u8>,
}

#[allow(clippy::len_without_is_empty)]
impl Entropy {
    pub(crate) fn read(r: &mut dyn Read) -> Result<Entropy> {
        let mut raw = Vec::new();

        loop {
            let byte = r.read_u8()?;
            if byte != markers::P {
                raw.push(byte);
                continue;
            }

            let marker_byte = r.read_u8()?;
            match marker_byte {
                markers::EOI => {
                    return Ok(Entropy { raw });
                }
                _ => {
                    raw.push(byte);
                    raw.push(marker_byte);
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        // raw data + EOI marker (2 bytes)
        self.raw.len() + 2
    }

    pub(crate) fn write_to(&self, w: &mut dyn Write) -> Result<()> {
        w.write_all(&self.raw)?;

        w.write_u8(markers::P)?;
        w.write_u8(markers::EOI)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_entropy() {
        let input: &[u8] = &[
            0xE2, 0xFF, 0xE2, 0x51, 0xE7, 0xFF, 0x00, 0xAA, 0xFD, 0xFF, 0x00, 0xCA, 0xFF, 0xD9,
        ];

        let output = Entropy::read(&mut &input[..]).expect("read_entropy");
        assert_eq!(output.len(), input.len());
        assert_eq!(output.raw.as_slice(), &input[..input.len() - 2]);
    }

    #[test]
    fn test_write_entropy() {
        let mut raw = vec![0xE2, 0xFF, 0xE2, 0x51, 0xE7, 0xFF, 0xAA, 0xFD, 0xFF, 0xCA];

        let entropy = Entropy { raw: raw.clone() };

        let mut output = Vec::new();
        entropy.write_to(&mut output).expect("write_entropy");

        raw.push(markers::P);
        raw.push(markers::EOI);

        assert_eq!(entropy.len(), raw.len());
        assert_eq!(output.as_slice(), raw.as_slice());
    }
}
