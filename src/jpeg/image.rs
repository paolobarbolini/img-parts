use std::io::{self, Read, Write};

use byteorder::{ReadBytesExt, WriteBytesExt};

use super::markers;
use super::JpegSegment;
use crate::{Error, ImageEXIF, ImageICC, Result};

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
        let mut icc_parts: Vec<(u8, u8, &[u8])> = self
            .segments
            .iter()
            .filter_map(|segment| segment.icc())
            .collect();
        if icc_parts.is_empty() {
            return None;
        }

        // sort by seqno
        icc_parts.sort_by(|a, b| a.0.cmp(&b.0));

        let len = icc_parts.iter().map(|part| part.2.len()).sum::<usize>();
        let mut sequence = Vec::with_capacity(len);

        for part in icc_parts {
            sequence.extend(part.2);
        }

        Some(sequence)
    }

    fn set_icc_profile(&mut self, profile: Option<Vec<u8>>) {
        self.segments.retain(|segment| segment.icc().is_none());

        if let Some(profile) = profile {
            let segments_n = (profile.len() / ICC_SEGMENT_MAX_SIZE + 1) as u8;
            for i in 0..segments_n {
                let start = ICC_SEGMENT_MAX_SIZE * i as usize;
                let end = std::cmp::min(profile.len(), start + ICC_SEGMENT_MAX_SIZE);

                let segment = JpegSegment::new_icc(i + 1, segments_n, &profile[start..end]);
                self.segments.insert(3, segment);
            }
        }
    }
}

impl ImageEXIF for Jpeg {
    fn exif(&self) -> Option<Vec<u8>> {
        self.segments
            .iter()
            .find_map(|segment| segment.exif().map(|buf| buf.to_vec()))
    }

    fn set_exif(&mut self, exif: Option<Vec<u8>>) {
        self.segments.retain(|segment| segment.exif().is_none());

        if let Some(exif) = exif {
            let segment = JpegSegment::new_exif(&exif);
            self.segments.insert(3, segment);
        }
    }
}
