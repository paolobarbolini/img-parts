use std::io::{self, Read, Write};

use byteorder::{ReadBytesExt, WriteBytesExt};

use super::markers;
use super::JpegSegment;
use crate::{Error, ImageICC, Result};

const ICC_DATA_PREFIX: &[u8] = b"ICC_PROFILE\0";
// max chunk size: u16::max_value() - segment size (2 byte) - segment meta (14 byte)
const ICC_SEGMENT_MAX_SIZE: usize = 65535 - 2 - 14;

/// The representation of a Jpeg image.
#[derive(Debug, Clone, PartialEq)]
pub struct Jpeg {
    segments: Vec<JpegSegment>,
}

#[allow(clippy::len_without_is_empty)]
impl Jpeg {
    /// Create a new `Jpeg` image from a Reader.
    ///
    /// # Errors
    ///
    /// This method fails if reading fails or if the first two bytes
    /// aren't a SOI marker.
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

    /// Get the segments of this `Jpeg`.
    #[inline]
    pub fn segments(&self) -> &Vec<JpegSegment> {
        &self.segments
    }

    /// Get a mutable reference to the segments of this `Jpeg`.
    #[inline]
    pub fn segments_mut(&mut self) -> &mut Vec<JpegSegment> {
        &mut self.segments
    }

    /// Get the first segment with a marker of `marker`
    pub fn segment_by_marker(&self, marker: u8) -> Option<&JpegSegment> {
        self.segments
            .iter()
            .find(|segment| segment.marker() == marker)
    }

    /// Get every segment with a marker of `marker`
    pub fn segments_by_marker(&self, marker: u8) -> Vec<&JpegSegment> {
        self.segments
            .iter()
            .filter(|segment| segment.marker() == marker)
            .collect()
    }

    /// Get the total size of the `Jpeg` once it is encoded.
    ///
    /// The size is the sum of:
    ///
    /// - The SOI marker (2 bytes).
    /// - The size of every segment including the size of the encoded entropy.
    pub fn len(&self) -> usize {
        // SOI marker (2 bytes) + length of every segment including entropy
        2 + self
            .segments
            .iter()
            .map(|segment| segment.len_with_entropy())
            .sum::<usize>()
    }

    /// Encode this `Jpeg` and write it to a Writer
    pub fn write_to(&self, w: &mut dyn Write) -> io::Result<()> {
        w.write_u8(markers::P)?;
        w.write_u8(markers::SOI)?;

        for segment in &self.segments {
            segment.write_to(w)?;
        }

        Ok(())
    }
}

impl ImageICC for Jpeg {
    fn icc_profile(&self) -> Option<Vec<u8>> {
        let app2s = self.segments_by_marker(markers::APP2);
        if app2s.is_empty() {
            return None;
        }

        let mut app2s_n = app2s.len();

        let mut total_len = 0;
        let mut sequences = Vec::with_capacity(app2s_n);
        for app2 in app2s {
            let contents = app2.contents();
            if contents.get(0..12) != Some(ICC_DATA_PREFIX) {
                app2s_n -= 1;
                continue;
            }

            let seqno = *contents.get(12)? as usize;
            if seqno == 0 || seqno > app2s_n {
                // TODO: invalid sequence number
                return None;
            }

            let num = *contents.get(13)? as usize;
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

    fn set_icc_profile(&mut self, profile: Option<Vec<u8>>) {
        self.segments.retain(|segment| {
            segment.marker() != markers::APP2
                || segment.contents().get(0..12) != Some(ICC_DATA_PREFIX)
        });

        if let Some(profile) = profile {
            let segments_n = profile.len() / ICC_SEGMENT_MAX_SIZE + 1;
            for i in 0..segments_n {
                let start = ICC_SEGMENT_MAX_SIZE * i;
                let end = std::cmp::min(profile.len(), start + ICC_SEGMENT_MAX_SIZE);
                let len = end - start;

                let mut contents = Vec::with_capacity(len + 16);
                contents.extend(ICC_DATA_PREFIX);
                contents.push(i as u8 + 1);
                contents.push(segments_n as u8);
                contents.extend(profile.get(start..end).unwrap());

                let segment = JpegSegment::new_with_contents(markers::APP2, contents);
                self.segments.insert(3, segment);
            }
        }
    }
}
