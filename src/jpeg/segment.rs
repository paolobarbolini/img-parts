use core::fmt;

use bytes::{Buf, BufMut, Bytes, BytesMut};

use super::markers::{self, has_entropy, has_length};
use crate::encoder::{EncodeAt, ImageEncoder};
use crate::util::{read_checked, split_to_checked};
use crate::{Error, Result, EXIF_DATA_PREFIX};

const ICC_DATA_PREFIX: &[u8] = b"ICC_PROFILE\0";

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
    #[inline]
    pub fn new_with_contents(marker: u8, contents: Bytes) -> JpegSegment {
        JpegSegment {
            marker,
            contents,
            entropy: Bytes::new(),
        }
    }

    /// Construct a `JpegSegment` with `contents` and `entropy`
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
    /// the `Entropy`.
    ///
    /// The size is the sum of:
    ///
    /// - The marker (2 bytes).
    /// - The length (2 bytes) if this marker has a length.
    /// - The size of the content.
    /// - The size of the encoded entropy data.
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
        let seqno = b.get_u8();
        // number of sequences
        let num = b.get_u8();

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

    /// Create an [encoder][crate::ImageEncoder] for this `JpegSegment`
    #[inline]
    pub fn encoder(self) -> ImageEncoder<Self> {
        ImageEncoder::from(self)
    }
}

impl EncodeAt for JpegSegment {
    fn encode_at(&self, pos: &mut usize) -> Option<Bytes> {
        match pos {
            0 => {
                let mut vec = BytesMut::with_capacity(4);
                vec.put_u8(markers::P);
                vec.put_u8(self.marker());
                vec.put_u16((self.len() - 2).try_into().unwrap());

                Some(vec.freeze())
            }
            1 if !self.contents.is_empty() => Some(self.contents.clone()),
            2 if !self.entropy.is_empty() => Some(self.entropy.clone()),
            _ => {
                let decrement =
                    1 + !self.contents.is_empty() as usize + !self.entropy.is_empty() as usize;
                *pos = pos.saturating_sub(decrement);
                None
            }
        }
    }

    fn len(&self) -> usize {
        self.len()
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
    use bytes::Bytes;

    #[test]
    fn test_encode_at_underflow() {
        let segment = JpegSegment::new_with_entropy(0xFF, Bytes::new(), Bytes::from(vec![4, 5, 6]));

        // This would cause an underflow if decrement not checked.
        let mut pos = 1;
        segment.encode_at(&mut pos);
    }
}
