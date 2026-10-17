use alloc::vec::Vec;

use bytes::{Buf, Bytes, BytesMut};

use super::{markers, mpf, JpegSegment};
use crate::{
    encoder::{Encode, ImageEncoder},
    util::read_checked,
    Error, ImageEXIF, ImageICC, Result, EXIF_DATA_PREFIX,
};

// segment size (2 byte) - segment meta (14 byte)
pub const ICC_PREFIX_SIZE: usize = 2 + 14;

// max chunk size: u16::max_value() - ICC_PREFIX_SIZE
const ICC_SEGMENT_MAX_SIZE: usize = 65535 - ICC_PREFIX_SIZE;

// max ICC profile size: u8::max_value() chunks
const ICC_MAX_SIZE: usize = 255 * ICC_SEGMENT_MAX_SIZE;

// max EXIF size: u16::max_value() - segment size (2 byte) - EXIF prefix
const EXIF_MAX_SIZE: usize = 65535 - 2 - EXIF_DATA_PREFIX.len();

/// The representation of a Jpeg image.
#[derive(Debug, Clone, PartialEq, Eq)]
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

            let marker = loop {
                let marker = read_checked(&mut b, |b| b.get_u8())?;
                if marker != markers::P {
                    break marker;
                }
            };

            if marker == markers::EOI {
                break;
            }

            // markers without a length carry no data,
            // so they're skipped just like fill bytes
            if !markers::has_length(marker) {
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
    /// - The EOI marker (2 bytes) if no segment has entropy, which otherwise
    ///   includes it.
    pub fn len(&self) -> usize {
        // SOI marker (2 bytes) + length of every segment including entropy
        let len = 2 + self
            .segments
            .iter()
            .map(|segment| segment.len_with_entropy())
            .sum::<usize>();

        // EOI marker (2 bytes)
        if self.has_entropy() {
            len
        } else {
            len + 2
        }
    }

    /// Create an [encoder][crate::ImageEncoder] for this `Jpeg`
    #[inline]
    pub fn encoder(self) -> ImageEncoder<Self> {
        ImageEncoder::from(self)
    }

    /// Check if any segment has entropy, which then includes the EOI marker
    fn has_entropy(&self) -> bool {
        self.segments.iter().any(JpegSegment::has_entropy)
    }

    /// Get the size of the segments before and after the MPF segment
    fn mpf_split(&self) -> Option<(usize, usize)> {
        let pos = self.segments.iter().position(JpegSegment::is_mpf)?;
        let len = |segments: &[JpegSegment]| segments.iter().map(JpegSegment::len).sum::<usize>();
        Some((len(&self.segments[..pos]), len(&self.segments[pos + 1..])))
    }

    /// Updates the MPF segment after the segments around it changed size,
    /// given their `old` size from [`Jpeg::mpf_split`], so that the offsets
    /// of the other images it lists stay valid
    fn update_mpf(&mut self, old: Option<(usize, usize)>) {
        let ((old_before, old_after), (before, after)) = match (old, self.mpf_split()) {
            (Some(old), Some(new)) => (old, new),
            _ => return,
        };
        // the other images move along with the segments after the MPF segment
        let offset_delta = after as i64 - old_after as i64;
        let size_delta = offset_delta + (before as i64 - old_before as i64);
        if size_delta == 0 && offset_delta == 0 {
            return;
        }

        if let Some(segment) = self.segments.iter_mut().find(|segment| segment.is_mpf()) {
            if let Some(contents) =
                mpf::update_entries(segment.contents(), size_delta, offset_delta)
            {
                *segment = JpegSegment::new_with_contents(markers::APP2, contents);
            }
        }
    }

    /// Get the position of the frame header, or of the scan header if
    /// there's none, which metadata segments must be inserted before
    fn frame_header_position(&self) -> usize {
        self.segments
            .iter()
            .position(|segment| {
                markers::is_sof(segment.marker()) || segment.marker() == markers::SOS
            })
            .unwrap_or(self.segments.len())
    }
}

impl Encode for Jpeg {
    fn encode(&self, pieces: &mut Vec<Bytes>) {
        pieces.push(Bytes::from_static(&[markers::P, markers::SOI]));

        for segment in &self.segments {
            segment.encode(pieces);
        }

        if !self.has_entropy() {
            pieces.push(Bytes::from_static(&[markers::P, markers::EOI]));
        }
    }
}

impl ImageICC for Jpeg {
    fn icc_profile(&self) -> Option<Bytes> {
        let mut icc_parts: Vec<(u8, u8, Bytes)> = self
            .segments
            .iter()
            .filter_map(|segment| segment.icc())
            .collect();

        // sort by seqno
        icc_parts.sort_by_key(|&(seqno, _, _)| seqno);

        // like libjpeg, ignore profiles with missing, duplicated or inconsistent segments
        let count = icc_parts.len();
        let valid = icc_parts
            .iter()
            .enumerate()
            .all(|(i, &(seqno, num, _))| usize::from(seqno) == i + 1 && usize::from(num) == count);
        if count == 0 || !valid {
            return None;
        }

        if count == 1 {
            return icc_parts.pop().map(|(_, _, part)| part);
        }

        let len = icc_parts.iter().map(|part| part.2.len()).sum::<usize>();
        let mut sequence = BytesMut::with_capacity(len);

        for part in icc_parts {
            sequence.extend_from_slice(&part.2);
        }

        Some(sequence.freeze())
    }

    fn set_icc_profile(&mut self, profile: Option<Bytes>) {
        let mpf = self.mpf_split();
        self.segments.retain(|segment| segment.icc().is_none());

        // ICC profiles that don't fit into 255 segments can't be stored
        if let Some(profile) = profile.filter(|profile| profile.len() <= ICC_MAX_SIZE) {
            // after the first 3 segments, but before the frame header
            let pos = self.frame_header_position().min(3);
            // at least 1, even for an empty profile
            let segments_n = (profile.len().saturating_sub(1) / ICC_SEGMENT_MAX_SIZE + 1) as u8;
            let segments = (0..segments_n).map(|i| {
                let start = ICC_SEGMENT_MAX_SIZE * i as usize;
                let end = core::cmp::min(profile.len(), start + ICC_SEGMENT_MAX_SIZE);

                JpegSegment::new_icc(i + 1, segments_n, profile.slice(start..end))
            });
            self.segments.splice(pos..pos, segments);
        }

        self.update_mpf(mpf);
    }
}

impl ImageEXIF for Jpeg {
    fn exif(&self) -> Option<Bytes> {
        self.segments.iter().find_map(|segment| segment.exif())
    }

    fn set_exif(&mut self, exif: Option<Bytes>) {
        let mpf = self.mpf_split();
        let old_pos = self
            .segments
            .iter()
            .position(|segment| segment.exif().is_some());
        self.segments.retain(|segment| segment.exif().is_none());

        // EXIF metadata that doesn't fit into a single segment can't be stored
        if let Some(exif) = exif.filter(|exif| exif.len() <= EXIF_MAX_SIZE) {
            // where the old EXIF segment was, so that the other segments don't
            // move, or right after the SOI marker and any APP0 segments, like
            // the JFIF one, but before the frame header
            let pos = old_pos
                .unwrap_or_else(|| {
                    self.segments
                        .iter()
                        .position(|segment| segment.marker() != markers::APP0)
                        .unwrap_or(self.segments.len())
                })
                .min(self.frame_header_position());
            let segment = JpegSegment::new_exif(exif);
            self.segments.insert(pos, segment);
        }

        self.update_mpf(mpf);
    }
}

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};

    use super::*;

    #[test]
    fn set_metadata_few_segments() {
        let mut jpeg = Jpeg::from_bytes(Bytes::from_static(&[0xFF, 0xD8, 0xFF, 0xD9])).unwrap();
        // big enough to be split into two segments
        let profile = Bytes::from((0..70000).map(|i| i as u8).collect::<Vec<u8>>());
        let exif = Bytes::from_static(b"exif");
        jpeg.set_icc_profile(Some(profile.clone()));
        jpeg.set_exif(Some(exif.clone()));
        assert_eq!(jpeg.icc_profile(), Some(profile));
        assert_eq!(jpeg.exif(), Some(exif));
    }

    #[test]
    fn set_icc_profile_order() {
        let mut jpeg = Jpeg::from_bytes(Bytes::from_static(&[0xFF, 0xD8, 0xFF, 0xD9])).unwrap();
        jpeg.set_icc_profile(Some(vec![0; 2 * ICC_SEGMENT_MAX_SIZE + 1].into()));

        let order: Vec<(u8, u8)> = jpeg
            .segments()
            .iter()
            .filter_map(|segment| segment.icc())
            .map(|(seqno, num, _)| (seqno, num))
            .collect();
        assert_eq!(order, [(1, 3), (2, 3), (3, 3)]);
    }

    #[test]
    fn invalid_icc_segments() {
        let icc = |seqno, num| JpegSegment::new_icc(seqno, num, Bytes::from_static(b"icc"));
        for segments in [
            // invalid sequence number
            vec![icc(0, 1)],
            // missing segment
            vec![icc(1, 2)],
            // duplicated segment
            vec![icc(1, 1), icc(1, 1)],
            // inconsistent number of segments
            vec![icc(1, 2), icc(2, 3)],
        ] {
            let jpeg = Jpeg { segments };
            assert_eq!(jpeg.icc_profile(), None);
        }
    }

    #[test]
    fn set_icc_profile_size_limit() {
        let mut jpeg = Jpeg::from_bytes(Bytes::from_static(&[0xFF, 0xD8, 0xFF, 0xD9])).unwrap();

        jpeg.set_icc_profile(Some(vec![0; ICC_SEGMENT_MAX_SIZE].into()));
        assert_eq!(jpeg.segments().len(), 1);

        jpeg.set_icc_profile(Some(vec![0; ICC_MAX_SIZE].into()));
        assert_eq!(jpeg.segments().len(), 255);
        assert_eq!(
            jpeg.icc_profile().map(|profile| profile.len()),
            Some(ICC_MAX_SIZE)
        );

        for len in [ICC_MAX_SIZE + 1, 2 * ICC_MAX_SIZE] {
            jpeg.set_icc_profile(Some(vec![0; len].into()));
            assert!(jpeg.icc_profile().is_none());
        }
    }

    #[test]
    fn markers_length() {
        // a DNL segment, which has a length, and an RST0 marker, which doesn't
        let file = Bytes::from_static(&[
            0xFF, 0xD8, 0xFF, 0xDC, 0x00, 0x04, 0x00, 0x10, 0xFF, 0xD0, 0xFF, 0xDB, 0x00, 0x03,
            0x00, 0xFF, 0xD9,
        ]);
        let jpeg = Jpeg::from_bytes(file).unwrap();
        let dnl = jpeg.segment_by_marker(markers::DNL).unwrap();
        assert_eq!(dnl.contents(), &Bytes::from_static(&[0x00, 0x10]));
        assert!(jpeg.segment_by_marker(markers::DQT).is_some());
    }

    #[test]
    fn skip_markers_without_length() {
        // TEM, RST0 and a repeated SOI before a DQT segment
        let file = Bytes::from_static(&[
            0xFF, 0xD8, 0xFF, 0x01, 0xFF, 0xD0, 0xFF, 0xD8, 0xFF, 0xDB, 0x00, 0x03, 0x00, 0xFF,
            0xD9,
        ]);
        let jpeg = Jpeg::from_bytes(file).unwrap();
        let markers: Vec<u8> = jpeg.segments().iter().map(JpegSegment::marker).collect();
        assert_eq!(markers, [markers::DQT]);
    }

    #[test]
    fn encode_without_entropy() {
        for file in [
            &[0xFF, 0xD8, 0xFF, 0xD9][..],
            &[0xFF, 0xD8, 0xFF, 0xFE, 0x00, 0x04, b'h', b'i', 0xFF, 0xD9],
        ] {
            let jpeg = Jpeg::from_bytes(Bytes::from_static(file)).unwrap();
            assert_eq!(jpeg.len(), file.len());
            assert_eq!(jpeg.encoder().bytes(), file);
        }
    }

    #[test]
    fn set_metadata_before_frame_header() {
        use markers::{APP0, APP1, APP2, DHT, DQT, SOF0, SOF2, SOS};

        for (layout, expected) in [
            (
                &[APP0, SOF0, DQT, SOS][..],
                &[APP0, APP1, APP2, SOF0, DQT, SOS][..],
            ),
            (
                &[APP0, DQT, SOF2, DHT, SOS],
                &[APP0, APP1, DQT, APP2, SOF2, DHT, SOS],
            ),
            // no frame header before the scan
            (&[APP0, SOS], &[APP0, APP1, APP2, SOS]),
        ] {
            let segments = layout
                .iter()
                .map(|&marker| JpegSegment::new_with_contents(marker, Bytes::from_static(b"data")))
                .collect();
            let mut jpeg = Jpeg { segments };
            jpeg.set_icc_profile(Some(Bytes::from_static(b"icc")));
            jpeg.set_exif(Some(Bytes::from_static(b"exif")));

            let result: Vec<u8> = jpeg.segments().iter().map(JpegSegment::marker).collect();
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn set_exif_position() {
        use markers::{APP0, APP1, APP2, DQT, SOF0};

        let segment = JpegSegment::new_with_contents;
        let jfif = segment(APP0, Bytes::from_static(b"JFIF\0"));
        let exif = JpegSegment::new_exif(Bytes::from_static(b"old"));
        let xmp = segment(APP1, Bytes::from_static(b"http://ns.adobe.com/xap/1.0/\0"));
        let mpf = segment(APP2, Bytes::from_static(b"MPF\0"));
        let dqt = segment(DQT, Bytes::from_static(b"dqt"));
        let sof = segment(SOF0, Bytes::from_static(b"sof"));
        for (segments, pos) in [
            // before the XMP segment, which would hide it from some readers
            (vec![jfif.clone(), xmp.clone(), dqt.clone()], 1),
            // where it was, since moving it across the MPF segment breaks its offsets
            (vec![exif.clone(), xmp, mpf.clone(), dqt.clone()], 0),
            (vec![jfif.clone(), mpf.clone(), exif.clone(), dqt], 2),
            // but before the frame header
            (vec![jfif, mpf, sof, exif], 2),
        ] {
            let mut jpeg = Jpeg { segments };
            jpeg.set_exif(Some(Bytes::from_static(b"exif")));
            let exif_pos = jpeg.segments().iter().position(|s| s.exif().is_some());
            assert_eq!(exif_pos, Some(pos));
        }
    }

    #[test]
    fn update_mpf_offsets() {
        // the first image, with an MPF segment pointing to a second one right after it
        let segment = |marker: u8, contents: &[u8]| {
            let mut segment = vec![0xFF, marker];
            segment.extend_from_slice(&(contents.len() as u16 + 2).to_be_bytes());
            segment.extend_from_slice(contents);
            segment
        };
        let exif = segment(markers::APP1, b"Exif\0\0exif");
        let tables = [segment(markers::DQT, &[0]), segment(markers::SOS, &[0])].concat();
        let scan = b"scan\xFF\xD9";
        let mpf = |size: u32, offset: u32| {
            let mut contents = b"MPF\0II*\0\x08\0\0\0\x01\0\x02\xB0\x07\0".to_vec();
            contents.extend_from_slice(b"\x20\0\0\0\x1A\0\0\0\0\0\0\0");
            for (size, offset) in [(size, 0), (4, offset)] {
                contents.extend_from_slice(&[0; 4]);
                contents.extend_from_slice(&size.to_le_bytes());
                contents.extend_from_slice(&offset.to_le_bytes());
                contents.extend_from_slice(&[0; 4]);
            }
            segment(markers::APP2, &contents)
        };
        let first_len = 2 + exif.len() + mpf(0, 0).len() + tables.len() + scan.len();
        let header = 2 + exif.len() + 8;
        let mpf = mpf(first_len as u32, (first_len - header) as u32);
        let file = [
            &[0xFF, 0xD8],
            &exif[..],
            &mpf,
            &tables,
            scan,
            &[0xFF, 0xD8, 0xFF, 0xD9],
        ];
        let jpeg = Jpeg::from_bytes(Bytes::from(file.concat())).unwrap();

        let profile = Bytes::from(vec![0; 70000]);
        let exif = Bytes::from_static(b"bigger exif");
        let edits: [fn(&mut Jpeg, Bytes, Bytes); 4] = [
            |jpeg, profile, _| jpeg.set_icc_profile(Some(profile)),
            |jpeg, _, exif| jpeg.set_exif(Some(exif)),
            |jpeg, _, _| jpeg.set_exif(None),
            |jpeg, profile, _| {
                jpeg.set_icc_profile(Some(profile));
                jpeg.set_icc_profile(None);
            },
        ];
        for edit in edits {
            let mut jpeg = jpeg.clone();
            edit(&mut jpeg, profile.clone(), exif.clone());
            let file = jpeg.encoder().bytes();

            let header = file.windows(4).position(|w| w == b"MPF\0").unwrap() + 4;
            let read = |pos: usize| u32::from_le_bytes(file[pos..pos + 4].try_into().unwrap());
            let (size, offset) = (read(header + 30) as usize, read(header + 50) as usize);
            // the second image is still at the end of the file
            assert_eq!(header + offset, file.len() - 4);
            assert_eq!(size, header + offset);
        }
    }

    #[test]
    fn set_exif_size_limit() {
        let mut jpeg = Jpeg::from_bytes(Bytes::from_static(&[0xFF, 0xD8, 0xFF, 0xD9])).unwrap();

        jpeg.set_exif(Some(vec![0; EXIF_MAX_SIZE].into()));
        assert_eq!(jpeg.exif().map(|exif| exif.len()), Some(EXIF_MAX_SIZE));
        assert_eq!(jpeg.clone().encoder().bytes().len(), jpeg.len());

        jpeg.set_exif(Some(vec![0; EXIF_MAX_SIZE + 1].into()));
        assert!(jpeg.exif().is_none());
    }

    #[test]
    fn encode_entropy_without_contents() {
        let file = Bytes::from_static(&[0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x02, 4, 5, 6, 0xFF, 0xD9]);
        let jpeg = Jpeg::from_bytes(file.clone()).unwrap();
        assert_eq!(jpeg.encoder().bytes(), file);
    }

    #[test]
    fn truncated_icc_segment() {
        for contents in [&b"ICC_PROFILE\0"[..], b"ICC_PROFILE\0\x01"] {
            let segment =
                JpegSegment::new_with_contents(markers::APP2, Bytes::from_static(contents));
            let mut jpeg = Jpeg {
                segments: vec![segment],
            };
            assert_eq!(jpeg.icc_profile(), None);

            // too short to be an ICC segment, so it isn't removed
            jpeg.set_icc_profile(None);
            assert_eq!(jpeg.segments().len(), 1);
        }
    }
}
