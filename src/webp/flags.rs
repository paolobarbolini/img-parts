use super::{WebP, CHUNK_EXIF, CHUNK_ICCP};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct WebPFlags(pub(super) [u8; 4]);

impl WebPFlags {
    pub(super) fn from_webp(webp: &WebP) -> WebPFlags {
        let mut flags = WebPFlags::default();
        if webp.has_chunk(CHUNK_ICCP) {
            flags.0[0] |= 0b0010_0000;
        }
        if webp.has_chunk(CHUNK_EXIF) {
            flags.0[0] |= 0b0000_1000;
        }
        flags
    }
}
