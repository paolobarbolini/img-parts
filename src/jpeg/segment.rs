use alloc::vec::Vec;
use core::fmt;

use bytes::{Buf, BufMut, Bytes, BytesMut};

use super::markers::{self, has_entropy, has_length};
use crate::{
    encoder::{Encode, ImageEncoder},
    util::{read_checked, split_to_checked},
    Error, Result, EXIF_DATA_PREFIX,
};

const ICC_DATA_PREFIX: &[u8] = b"ICC_PROFILE\0";
const MPF_DATA_PREFIX: &[u8] = b"MPF\0";

/// The representation of a segment making up a [`Jpeg`][super::Jpeg]
#[derive(Clone, PartialEq, Eq)]
pub struct JpegSegment {
    marker: u8,
    contents: Bytes,
    entropy: Bytes,
}

#[allow(clippy::len_without_is_empty)]
impl JpegSegment {
    /// Construct an empty `JpegSegment`
    #[inline]
    pub fn new(marker: u8) -> JpegSegment {
        JpegSegment {
            marker,
            contents: Bytes::new(),
            entropy: Bytes::new(),
        }
    }

    /// Construct a `JpegSegment` with `contents`
    ///
    /// Encoding it panics if `contents` is bigger than 65533 bytes,
    /// the most a segment can hold.
    #[inline]
    pub fn new_with_contents(marker: u8, contents: Bytes) -> JpegSegment {
        JpegSegment {
            marker,
            contents,
            entropy: Bytes::new(),
        }
    }

    /// Construct a `JpegSegment` with `contents` and `entropy`
    ///
    /// Encoding it panics if `contents` is bigger than 65533 bytes,
    /// the most a segment can hold.
    #[inline]
    pub fn new_with_entropy(marker: u8, contents: Bytes, entropy: Bytes) -> JpegSegment {
        JpegSegment {
            marker,
            contents,
            entropy,
        }
    }

    /// Creates an ICC `JpegSegment`
    pub(super) fn new_icc(seqno: u8, num: u8, buf: Bytes) -> JpegSegment {
        let mut contents = BytesMut::with_capacity(ICC_DATA_PREFIX.len() + 2 + buf.len());
        contents.put(ICC_DATA_PREFIX);
        contents.put_u8(seqno);
        contents.put_u8(num);
        contents.put(buf);

        JpegSegment::new_with_contents(markers::APP2, contents.freeze())
    }

    /// Creates an EXIF `JpegSegment`
    pub(super) fn new_exif(buf: Bytes) -> JpegSegment {
        let mut contents = BytesMut::with_capacity(EXIF_DATA_PREFIX.len() + buf.len());
        contents.put(EXIF_DATA_PREFIX);
        contents.put(buf);

        JpegSegment::new_with_contents(markers::APP1, contents.freeze())
    }

    pub(crate) fn from_bytes(marker: u8, b: &mut Bytes) -> Result<JpegSegment> {
        let size = read_checked(b, |b| b.get_u16())?
            .checked_sub(2)
            .ok_or(Error::Truncated)?;
        let contents = split_to_checked(b, size as usize)?;

        if !has_entropy(marker) {
            Ok(JpegSegment::new_with_contents(marker, contents))
        } else {
            Ok(JpegSegment::new_with_entropy(marker, contents, b.clone()))
        }
    }

    /// Get the size of this `JpegSegment` once it is encoded, entropy
    /// excluded.
    ///
    /// The size is the sum of:
    ///
    /// - The marker (2 bytes).
    /// - The length (2 bytes) if this marker has a length.
    /// - The size of the content.
    pub fn len(&self) -> usize {
        if has_length(self.marker) {
            // 2 bytes (marker) + 2 bytes (length) + length of the content
            2 + 2 + self.contents.len()
        } else {
            // 2 bytes (marker) + length of the content
            2 + self.contents.len()
        }
    }

    /// Get the size of this `JpegSegment` once it is encoded, including
    /// the entropy.
    ///
    /// The size is the sum of:
    ///
    /// - The marker (2 bytes).
    /// - The length (2 bytes) if this marker has a length.
    /// - The size of the content.
    /// - The size of the encoded entropy data.
    ///
    /// For the SOS segment of a parsed [`Jpeg`][super::Jpeg], the entropy
    /// includes the rest of the file.
    pub fn len_with_entropy(&self) -> usize {
        self.len() + self.entropy.len()
    }

    /// Get the second byte of the marker of this `JpegSegment`
    #[inline]
    pub fn marker(&self) -> u8 {
        self.marker
    }

    /// Get the content of this `JpegSegment`
    #[inline]
    pub fn contents(&self) -> &Bytes {
        &self.contents
    }

    /// Check if this `JpegSegment` has entropy
    #[inline]
    pub fn has_entropy(&self) -> bool {
        !self.entropy.is_empty()
    }

    /// Returns the ICC segment data if this `JpegSegment` is an ICC segment.
    pub(super) fn icc(&self) -> Option<(u8, u8, Bytes)> {
        if self.marker != markers::APP2 || !self.contents.starts_with(ICC_DATA_PREFIX) {
            return None;
        }

        let mut b = self.contents.slice(ICC_DATA_PREFIX.len()..);

        // sequence number (between 1 and N. of sequence numbers inclusive)
        let seqno = b.try_get_u8().ok()?;
        // number of sequences
        let num = b.try_get_u8().ok()?;

        Some((seqno, num, b))
    }

    /// Returns the EXIF segment data if this `JpegSegment` is an EXIF segment.
    pub(super) fn exif(&self) -> Option<Bytes> {
        if self.marker == markers::APP1 && self.contents.starts_with(EXIF_DATA_PREFIX) {
            Some(self.contents.slice(EXIF_DATA_PREFIX.len()..))
        } else {
            None
        }
    }

    /// Check if this `JpegSegment` is an MPF segment.
    pub(super) fn is_mpf(&self) -> bool {
        self.marker == markers::APP2 && self.contents.starts_with(MPF_DATA_PREFIX)
    }

    /// Create an [encoder][crate::ImageEncoder] for this `JpegSegment`
    #[inline]
    pub fn encoder(self) -> ImageEncoder<Self> {
        ImageEncoder::from(self)
    }
}

impl Encode for JpegSegment {
    fn encode(&self, pieces: &mut Vec<Bytes>) {
        let mut vec = BytesMut::with_capacity(4);
        vec.put_u8(markers::P);
        vec.put_u8(self.marker());
        if has_length(self.marker()) {
            vec.put_u16((self.len() - 2).try_into().unwrap());
        }
        pieces.push(vec.freeze());

        if !self.contents.is_empty() {
            pieces.push(self.contents.clone());
        }
        if !self.entropy.is_empty() {
            pieces.push(self.entropy.clone());
        }
    }
}

impl fmt::Debug for JpegSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JpegSegment")
            .field("marker", &self.marker)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_without_length() {
        // TEM, which has no length
        let segment = JpegSegment::new(0x01);
        assert_eq!(segment.len(), 2);
        assert_eq!(segment.encoder().bytes(), [markers::P, 0x01][..]);
    }
}
