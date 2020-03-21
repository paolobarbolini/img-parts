use std::io::{Read, Result, Write};

use byteorder::{ReadBytesExt, WriteBytesExt};

use super::markers;

pub fn read_entropy(r: &mut dyn Read) -> Result<Vec<u8>> {
    let mut entropy = Vec::new();

    loop {
        let byte = r.read_u8()?;
        if byte != markers::P {
            entropy.push(byte);
            continue;
        }

        let marker_byte = r.read_u8()?;
        match marker_byte {
            markers::EOI => {
                return Ok(entropy);
            }
            markers::Z => entropy.push(byte),
            _ => {
                entropy.push(byte);
                entropy.push(marker_byte);
            }
        }
    }
}

pub fn write_entropy(entropy: &[u8], w: &mut dyn Write) -> Result<()> {
    for byte in entropy {
        w.write_u8(*byte)?;

        if *byte == markers::P {
            w.write_u8(markers::Z)?;
        }
    }

    w.write_u8(markers::P)?;
    w.write_u8(markers::EOI)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_entropy() {
        let input: &[u8] = &[
            0xE2, 0x51, 0xE7, 0xFF, 0x00, 0xAA, 0xFD, 0xFF, 0x00, 0xCA, 0xFF, 0xD9,
        ];
        let expected: &[u8] = &[0xE2, 0x51, 0xE7, 0xFF, 0xAA, 0xFD, 0xFF, 0xCA];

        let output = read_entropy(&mut &input[..]).expect("read_entropy");
        assert_eq!(output.as_slice(), expected);
    }

    #[test]
    fn test_write_entropy() {
        let input: &[u8] = &[0xE2, 0x51, 0xE7, 0xFF, 0xAA, 0xFD, 0xFF, 0xCA];
        let expected: &[u8] = &[
            0xE2, 0x51, 0xE7, 0xFF, 0x00, 0xAA, 0xFD, 0xFF, 0x00, 0xCA, 0xFF, 0xD9,
        ];

        let mut output = Vec::new();
        write_entropy(input, &mut output).expect("write_entropy");
        assert_eq!(output.as_slice(), expected);
    }
}
