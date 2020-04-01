use std::io::{Read, Result, Write};

use byteorder::{ReadBytesExt, WriteBytesExt};

use super::markers;

#[derive(Clone, PartialEq)]
pub struct Entropy {
    data: Vec<u8>,
    markers: Vec<usize>,
}

#[allow(clippy::len_without_is_empty)]
impl Entropy {
    pub(crate) fn read(r: &mut dyn Read) -> Result<Entropy> {
        let mut pos = 0;
        let mut data = Vec::new();
        let mut markers = Vec::new();

        loop {
            let byte = r.read_u8()?;
            if byte != markers::P {
                pos += 1;
                data.push(byte);
                continue;
            }

            let marker_byte = r.read_u8()?;
            match marker_byte {
                markers::EOI => {
                    return Ok(Entropy { data, markers });
                }
                markers::Z => {
                    pos += 1;
                    data.push(byte)
                }
                _ => {
                    markers.push(pos);
                    data.push(byte);
                    data.push(marker_byte);
                    pos += 2;
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        // data + # byte stuffing bytes + EOI marker (2 bytes)
        self.data.len() + bytecount::count(&self.data, markers::P) - self.markers.len() + 2
    }

    pub(crate) fn write_to(&self, w: &mut dyn Write) -> Result<()> {
        for (pos, byte) in self.data.iter().enumerate() {
            w.write_u8(*byte)?;

            if *byte == markers::P && !self.markers.contains(&pos) {
                w.write_u8(markers::Z)?;
            }
        }

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
        let expected_data: &[u8] = &[0xE2, 0xFF, 0xE2, 0x51, 0xE7, 0xFF, 0xAA, 0xFD, 0xFF, 0xCA];
        let expected_markers: &[usize] = &[1];

        let output = Entropy::read(&mut &input[..]).expect("read_entropy");
        assert_eq!(output.len(), input.len());
        assert_eq!(output.data.as_slice(), expected_data);
        assert_eq!(output.markers.as_slice(), expected_markers);
    }

    #[test]
    fn test_write_entropy() {
        let data = vec![0xE2, 0xFF, 0xE2, 0x51, 0xE7, 0xFF, 0xAA, 0xFD, 0xFF, 0xCA];
        let markers = vec![1];
        let expected: &[u8] = &[
            0xE2, 0xFF, 0xE2, 0x51, 0xE7, 0xFF, 0x00, 0xAA, 0xFD, 0xFF, 0x00, 0xCA, 0xFF, 0xD9,
        ];

        let entropy = Entropy { data, markers };

        let mut output = Vec::new();
        entropy.write_to(&mut output).expect("write_entropy");
        assert_eq!(entropy.len(), expected.len());
        assert_eq!(output.as_slice(), expected);
    }
}
