use std::convert::TryInto;
use std::io::Read;

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};

use crate::riff::RiffChunk;
use crate::vp8::decode_size_vp8_from_header;
use crate::{Error, Result};

pub fn icc_from_webp(input: &mut dyn Read) -> Result<Vec<u8>> {
    let mut buf: [u8; 1024] = [0; 1024];
    input.read_exact(&mut buf[0..12])?;

    if &buf[8..12] != b"WEBP" {
        return Err(Error::NoWebpMarker);
    }

    input.read_exact(&mut buf[0..4])?;

    // check that this is an extended WebP
    if &buf[0..4] != b"VP8X" {
        return Err(Error::NoVP8X);
    }

    // check that the extended WebP header has the ICC bit
    let chunk = input.read_u32::<LittleEndian>().unwrap();
    let has_icc = (chunk & (1 << 3)) != 0;
    if !has_icc {
        return Err(Error::NoICC);
    }

    // skip canvas width (24 bits) + canvas height (24 bits) + empty (16 bits)
    input.read_exact(&mut buf[0..8])?;

    loop {
        input.read_exact(&mut buf[0..2])?;

        let chunk = RiffChunk::read(input)?;
        match chunk.id() {
            b"ICCP" => {
                return Ok(chunk.contents().to_vec());
            }
            b"VP8" => return Err(Error::ImageDataReached),
            _ => {}
        };
    }
}

pub fn add_icc_to_webp(input: &mut dyn Read, icc: Option<&[u8]>) -> Result<Vec<u8>> {
    let mut webp = Vec::new();

    let mut buf: [u8; 1024] = [0; 1024];
    input.read_exact(&mut buf[0..12])?;
    webp.extend(&buf[0..12]);

    if &buf[8..12] != b"WEBP" {
        return Err(Error::NoWebpMarker);
    }

    input.read_exact(&mut buf[0..4])?;

    // check that this is a simple webp or an extended webp
    if &buf[0..4] == b"VP8 " {
        webp.extend(b"VP8X");

        // read 'VP8 ' chunk size
        input.read_exact(&mut buf[0..4])?;

        // write extended WebP header
        // TODO: icc flag
        // 7 bit flags + 1 bit reserved + 24 bits reserved
        webp.extend(&[0x0a, 0x00, 0x00, 0x00]);

        // copied
        webp.extend(&[0x28, 0x00, 0x00, 0x00]);

        let mut header_buf: [u8; 10] = [0; 10];
        input.read_exact(&mut header_buf[..])?;
        let (width, height) = decode_size_vp8_from_header(&header_buf);

        // canvas width
        let mut u24_write = [0u8; 3];
        LittleEndian::write_u24(&mut u24_write, (width - 1).into()); // TODO: read from vp8
        webp.extend(&u24_write[..]);

        // canvas height
        let mut u24_write = [0u8; 3];
        LittleEndian::write_u24(&mut u24_write, (height - 1).into()); // TODO: read from vp8
        webp.extend(&u24_write[..]);

        // write ICCP
        write_iccp(&mut webp, icc);

        // write 'VP8 ' chunk
        webp.extend(b"VP8 ");

        // write 'VP8 ' chunk size
        webp.extend(&buf[0..4]);

        webp.extend(&header_buf[..]);
        input.read_to_end(&mut webp)?;
    } else if &buf[0..4] == b"VP8X" {
        webp.extend(b"VP8X");

        // read the extended WebP header
        // TODO: set ICC flag to 1
        input.read_exact(&mut buf[0..4])?;
        webp.extend(&buf[0..4]);

        // skip canvas width (24 bits) + canvas height (24 bits) + empty (16 bits)
        input.read_exact(&mut buf[0..8])?;
        webp.extend(&buf[0..8]);

        // what is this skipping, idk!
        input.read_exact(&mut buf[0..2])?;
        webp.extend(&buf[0..2]);

        // write ICCP
        write_iccp(&mut webp, icc);

        loop {
            let chunk = RiffChunk::read(input)?;
            let id = chunk.id();

            // remove pre-existing ICC profiles
            if id == b"ICCP" {
                continue;
            }

            let is_vp8 = id[0..3] == *b"VP8";

            webp.append(&mut chunk.bytes());

            if is_vp8 {
                break;
            }
        }
    } else {
        return Err(Error::NoVP8X);
    }

    input.read_to_end(&mut webp)?;

    // update the file size in the WebP File Header
    let len = webp.len();
    LittleEndian::write_u32(&mut webp[4..8], (len - 8).try_into().unwrap());

    Ok(webp)
}

fn write_iccp(webp: &mut Vec<u8>, icc: Option<&[u8]>) {
    if let Some(icc) = icc {
        let chunk = RiffChunk::new(*b"ICCP", icc.to_vec());
        webp.append(&mut chunk.bytes());
    }
}
