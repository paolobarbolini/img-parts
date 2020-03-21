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
            markers::Z => {}
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
            w.write_u8(0x00)?;
        }
    }

    w.write_u8(markers::EOI)?;
    Ok(())
}
