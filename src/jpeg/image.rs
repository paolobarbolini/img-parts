use std::io::{self, Read, Write};

use byteorder::{ReadBytesExt, WriteBytesExt};

use super::markers;
use super::JpegSegment;
use crate::{Error, Result};

pub struct Jpeg {
    segments: Vec<JpegSegment>,
}

impl Jpeg {
    pub fn read(r: &mut dyn Read) -> Result<Jpeg> {
        let b0 = r.read_u8()?;
        let b1 = r.read_u8()?;

        if b0 != markers::P || b1 != markers::SOI {
            return Err(Error::FirstTwoBytesNotSOI);
        }

        let mut segments = Vec::new();
        loop {
            let fmb = r.read_u8()?;
            if fmb != markers::P {
                continue;
            }

            let mut marker;
            loop {
                marker = r.read_u8()?;
                if marker != markers::P {
                    break;
                }
            }

            if marker == markers::EOI {
                break;
            }

            if !markers::has_length(marker) {
                let segment = JpegSegment::new(marker);
                segments.push(segment);
                continue;
            }

            let segment = JpegSegment::read(marker, r)?;
            let has_entropy = segment.has_entropy();
            segments.push(segment);

            if has_entropy {
                break;
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

    pub fn write_to(&self, w: &mut dyn Write) -> io::Result<()> {
        w.write_u8(markers::P)?;
        w.write_u8(markers::SOI)?;

        for segment in &self.segments {
            segment.write_to(w)?;
        }

        Ok(())
    }

    pub fn icc_profile(&self) -> Option<Vec<u8>> {
        let app2s = self.components_by_marker(markers::APP2);
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
