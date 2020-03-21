use std::convert::TryInto;
use std::io::{Read, Write};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::{Error, Result};

pub mod marker {
    // Byte stuffing
    pub const Z: u8 = 0x00;
    // First marker byte
    pub const P: u8 = 0xFF;

    // Markers

    // Start of Frame
    pub const SOF0: u8 = 0xC0;
    pub const SOF1: u8 = 0xC1;
    pub const SOF2: u8 = 0xC2;
    pub const SOF3: u8 = 0xC3;
    pub const DHT: u8 = 0xC4; // Define Huffman Table
    pub const SOF5: u8 = 0xC5;
    pub const SOF6: u8 = 0xC6;
    pub const SOF7: u8 = 0xC7;
    pub const JPG: u8 = 0xC8; // JPEG Extensions
    pub const SOF9: u8 = 0xC9;
    pub const SOF10: u8 = 0xCA;
    pub const SOF11: u8 = 0xCB;
    pub const DAC: u8 = 0xCC; // Define Arithmetic Coding
    pub const SOF13: u8 = 0xCD;
    pub const SOF14: u8 = 0xCE;
    pub const SOF15: u8 = 0xCF;

    // Restart Markers
    pub const RST0: u8 = 0xD0;
    pub const RST1: u8 = 0xD1;
    pub const RST2: u8 = 0xD2;
    pub const RST3: u8 = 0xD3;
    pub const RST4: u8 = 0xD4;
    pub const RST5: u8 = 0xD5;
    pub const RST6: u8 = 0xD6;
    pub const RST7: u8 = 0xD7;

    // {Start,End} of Image
    pub const SOI: u8 = 0xD8;
    pub const EOI: u8 = 0xD9;

    // Start of Scan
    pub const SOS: u8 = 0xDA;
    // Define Quantization Table
    pub const DQT: u8 = 0xDB;
    // Define Number of Lines
    pub const DNL: u8 = 0xDC;
    // Define Restart Interval
    pub const DRI: u8 = 0xDD;
    // Define Hiercarchical Progression
    pub const DHP: u8 = 0xDE;
    // Expand Reference Component
    pub const EXP: u8 = 0xDF;

    // Application Segments
    pub const APP0: u8 = 0xE0;
    pub const APP1: u8 = 0xE1;
    pub const APP2: u8 = 0xE2;
    pub const APP3: u8 = 0xE3;
    pub const APP4: u8 = 0xE4;
    pub const APP5: u8 = 0xE5;
    pub const APP6: u8 = 0xE6;
    pub const APP7: u8 = 0xE7;
    pub const APP8: u8 = 0xE8;
    pub const APP9: u8 = 0xE9;
    pub const APP10: u8 = 0xEA;
    pub const APP11: u8 = 0xEB;
    pub const APP12: u8 = 0xEC;
    pub const APP13: u8 = 0xED;
    pub const APP14: u8 = 0xEE;
    pub const APP15: u8 = 0xEF;

    // JPEG Extensions
    pub const JPG0: u8 = 0xF0;
    pub const JPG1: u8 = 0xF1;
    pub const JPG2: u8 = 0xF2;
    pub const JPG3: u8 = 0xF3;
    pub const JPG4: u8 = 0xF4;
    pub const JPG5: u8 = 0xF5;
    pub const JPG6: u8 = 0xF6;
    pub const JPG7: u8 = 0xF7;
    pub const JPG8: u8 = 0xF8;
    pub const JPG9: u8 = 0xF9;
    pub const JPG10: u8 = 0xFA;
    pub const JPG11: u8 = 0xFB;
    pub const JPG12: u8 = 0xFC;
    pub const JPG13: u8 = 0xFD;

    // Comment
    pub const COM: u8 = 0xFE;

    pub fn has_length(marker: u8) -> bool {
        match marker {
            RST0..=RST7 => true,
            APP1..=APP15 => true,
            SOF0..=SOF15 => true,
            SOS => true,
            COM => true,
            _ => false,
        }
    }

    pub fn has_entropy(marker: u8) -> bool {
        match marker {
            SOS => true,
            _ => false,
        }
    }
}

pub struct Jpeg {
    segments: Vec<JpegSegment>,
}

pub struct JpegSegment {
    marker: u8,
    contents: Vec<u8>,
    entropy_data: Option<Vec<u8>>,
}

impl Jpeg {
    pub fn read(r: &mut dyn Read) -> Result<Jpeg> {
        let b0 = r.read_u8()?;
        let b1 = r.read_u8()?;

        if b0 != marker::P || b1 != marker::SOI {
            return Err(Error::FirstTwoBytesNotSOI);
        }

        let mut segments = Vec::new();
        'main: loop {
            let fmb = r.read_u8()?;
            if fmb != marker::P {
                continue;
            }

            let marker = r.read_u8()?;

            match marker {
                marker::EOI => break,
                _ => {
                    if marker::has_length(marker) {
                        let mut segment = JpegSegment::read(marker, r)?;

                        if marker::has_entropy(marker) {
                            let mut entropy = Vec::new();

                            loop {
                                let byte = r.read_u8()?;

                                match byte {
                                    marker::P => {
                                        let byte2 = r.read_u8()?;

                                        match byte2 {
                                            marker::EOI => {
                                                segment.set_entropy_data(Some(entropy));
                                                segments.push(segment);
                                                break 'main;
                                            }
                                            marker::Z => {}
                                            _ => {
                                                entropy.push(byte);
                                                entropy.push(byte2);
                                            }
                                        }
                                    }
                                    _ => entropy.push(byte),
                                };
                            }
                        }

                        segments.push(segment);
                    }
                }
            }
        }

        Ok(Jpeg { segments })
    }

    #[inline]
    pub fn segments(&self) -> &[JpegSegment] {
        self.segments.as_slice()
    }

    #[inline]
    pub fn segments_mut(&mut self) -> &mut Vec<JpegSegment> {
        &mut self.segments
    }

    #[inline]
    pub fn component_by_marker(&self, marker: u8) -> Option<&JpegSegment> {
        self.segments
            .iter()
            .find(|segment| segment.marker() == marker)
    }

    #[inline]
    pub fn components_by_marker(&self, marker: u8) -> Vec<&JpegSegment> {
        self.segments
            .iter()
            .filter(|segment| segment.marker() == marker)
            .collect()
    }

    pub fn icc_profile(&self) -> Option<Vec<u8>> {
        let app2s = self.components_by_marker(marker::APP2);
        if app2s.is_empty() {
            return None;
        }

        let mut app2s_n = app2s.len();

        let mut total_len = 0;
        let mut sequences = Vec::with_capacity(app2s_n);
        for app2 in app2s {
            let contents = app2.contents();
            if contents.get(0..12) != Some(b"ICC_PROFILE\0") {
                app2s_n -= 1;
                continue;
            }

            let seqno = *contents.get(12).unwrap() as usize; // TODO: not enough bytes
            if seqno == 0 || seqno > app2s_n {
                // TODO: invalid sequence number
                return None;
            }

            let num = *contents.get(13).unwrap() as usize; // TODO: not enough bytes
            if num != app2s_n {
                // TODO: invalid number of markers
                return None;
            }

            let mut sequence = Vec::with_capacity(contents.len() - 14);
            sequence.extend(&contents[14..]);

            total_len += sequence.len();
            sequences.insert(seqno - 1, sequence);
        }

        if total_len == 0 {
            return None;
        }

        let mut final_sequence = Vec::with_capacity(total_len);
        for mut sequence in sequences {
            final_sequence.append(&mut sequence);
        }
        Some(final_sequence)
    }
}

impl JpegSegment {
    #[inline]
    pub fn new(marker: u8, contents: Vec<u8>) -> JpegSegment {
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

        Ok(JpegSegment::new(marker, contents))
    }

    #[inline]
    pub fn set_entropy_data(&mut self, entropy: Option<Vec<u8>>) {
        self.entropy_data = entropy;
    }

    pub fn size(&self) -> usize {
        // 2 bytes (marker) + 2 bytes (length) + length of the content
        2 + 2 + self.contents.len()
    }

    #[inline]
    pub fn marker(&self) -> u8 {
        self.marker
    }

    #[inline]
    pub fn contents(&self) -> &[u8] {
        self.contents.as_slice()
    }

    pub fn write_to(&self, w: &mut dyn Write) -> Result<()> {
        w.write_u16::<BigEndian>((self.contents.len() + 2).try_into().unwrap())?;
        w.write_all(&self.contents)?;

        Ok(())
    }
}
