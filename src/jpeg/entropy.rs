use std::io::{Read, Result};

use byteorder::ReadBytesExt;

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
