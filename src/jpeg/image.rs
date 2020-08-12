use std::io::{self, Write};

use bytes::{Buf, Bytes, BytesMut};

use super::markers;
use super::JpegSegment;
use crate::encoder::{EncodeAt, ImageEncoder};
use crate::util::read_checked;
use crate::{Error, ImageEXIF, ImageICC, Result};

// segment size (2 byte) - segment meta (14 byte)
pub const ICC_PREFIX_SIZE: usize = 2 + 14;

// max chunk size: u16::max_value() - ICC_PREFIX_SIZE
const ICC_SEGMENT_MAX_SIZE: usize = 65535 - ICC_PREFIX_SIZE;

/// The representation of a Jpeg image.
#[derive(Debug, Clone, PartialEq)]
pub struct Jpeg {
    segments: Vec<JpegSegment>,
}

#[allow(clippy::len_without_is_empty)]
impl Jpeg {
    /// Create a `Jpeg` from `Bytes`
    ///
    /// # Errors
    ///
    /// This method fails if the file signature doesn't match or if
    /// it is corrupted or truncated.
    pub fn from_bytes(mut b: Bytes) -> Result<Jpeg> {
        let b0 = read_checked(&mut b, |b| b.get_u8())?;
        let b1 = read_checked(&mut b, |b| b.get_u8())?;

        if b0 != markers::P || b1 != markers::SOI {
            return Err(Error::WrongSignature);
        }

        let mut segments = Vec::with_capacity(8);
        loop {
            let fmb = read_checked(&mut b, |b| b.get_u8())?;
            if fmb != markers::P {
                continue;
            }

            let mut marker;
            loop {
                marker = read_checked(&mut b, |b| b.get_u8())?;
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

            let segment = JpegSegment::from_bytes(marker, &mut b)?;
            let has_entropy = segment.has_entropy();
            segments.push(segment);

            if has_entropy {
                break;
            }
        }

        Ok(Jpeg { segments })
    }

    /// Get the segments of this `Jpeg`
    #[inline]
    pub fn segments(&self) -> &Vec<JpegSegment> {
        &self.segments
    }

    /// Get a mutable reference to the segments of this `Jpeg`
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
    pub fn segments_by_marker(&self, marker: u8) -> impl Iterator<Item = &JpegSegment> {
        self.segments
            .iter()
            .filter(move |segment| segment.marker() == marker)
    }

    /// Remove every segment with a marker of `marker`
    pub fn remove_segments_by_marker(&mut self, marker: u8) {
        self.segments.retain(|segment| segment.marker() != marker);
    }

    /// Get the total size of the `Jpeg` once it is encoded
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

    #[inline]
    #[doc(hidden)]
    #[deprecated(since = "0.2.0", note = "Please use Jpeg::encoder().write_to(writer)")]
    pub fn write_to(self, w: &mut dyn Write) -> io::Result<()> {
        self.encoder().write_to(w)?;
        Ok(())
    }

    /// Create an [encoder][crate::ImageEncoder] for this `Jpeg`
    #[inline]
    pub fn encoder(self) -> ImageEncoder<Self> {
        ImageEncoder::from(self)
    }
}

impl EncodeAt for Jpeg {
    fn encode_at(&self, pos: &mut usize) -> Option<Bytes> {
        match pos {
            0 => {
                let vec = Bytes::from_static(&[markers::P, markers::SOI]);
                Some(vec)
            }
            _ => {
                *pos -= 1;

                for segment in &self.segments {
                    if let Some(bytes) = segment.encode_at(pos) {
                        return Some(bytes);
                    }
                }

                None
            }
        }
    }

    fn len(&self) -> usize {
        self.len()
    }
}

impl ImageICC for Jpeg {
    fn icc_profile(&self) -> Option<Bytes> {
        let mut icc_parts = self.segments.iter().filter_map(|segment| segment.icc());

        let first = icc_parts.next()?;
        let second = match icc_parts.next() {
            Some(second) => second,
            None => return Some(first.2),
        };

        let mut icc_parts: Vec<(u8, u8, Bytes)> = icc_parts.collect();
        icc_parts.push(first);
        icc_parts.push(second);

        // sort by seqno
        icc_parts.sort_by(|a, b| a.0.cmp(&b.0));

        let len = icc_parts.iter().map(|part| part.2.len()).sum::<usize>();
        let mut sequence = BytesMut::with_capacity(len);

        for part in icc_parts {
            sequence.extend(part.2);
        }

        Some(sequence.freeze())
    }

    fn set_icc_profile(&mut self, profile: Option<Bytes>) {
        self.segments.retain(|segment| segment.icc().is_none());

        if let Some(profile) = profile {
            let segments_n = (profile.len() / ICC_SEGMENT_MAX_SIZE + 1) as u8;
            for i in 0..segments_n {
                let start = ICC_SEGMENT_MAX_SIZE * i as usize;
                let end = std::cmp::min(profile.len(), start + ICC_SEGMENT_MAX_SIZE);

                let segment = JpegSegment::new_icc(i + 1, segments_n, profile.slice(start..end));
                self.segments.insert(3, segment);
            }
        }
    }
}

impl ImageEXIF for Jpeg {
    fn exif(&self) -> Option<Bytes> {
        self.segments.iter().find_map(|segment| segment.exif())
    }

    fn set_exif(&mut self, exif: Option<Bytes>) {
        self.segments.retain(|segment| segment.exif().is_none());

        if let Some(exif) = exif {
            let segment = JpegSegment::new_exif(exif);
            self.segments.insert(3, segment);
        }
    }
}
