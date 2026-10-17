use super::{
    WebP, CHUNK_ALPH, CHUNK_ANIM, CHUNK_ANMF, CHUNK_EXIF, CHUNK_ICCP, CHUNK_VP8L, CHUNK_XMP,
};

const ICC: u8 = 0b0010_0000;
const ALPHA: u8 = 0b0001_0000;
const EXIF: u8 = 0b0000_1000;
const XMP: u8 = 0b0000_0100;
const ANIMATION: u8 = 0b0000_0010;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct WebPFlags(pub(super) [u8; 4]);

impl WebPFlags {
    pub(super) fn from_webp(webp: &WebP) -> WebPFlags {
        let mut flags = WebPFlags::default();
        if webp.has_chunk(CHUNK_ICCP) {
            flags.0[0] |= ICC;
        }
        let lossless_alpha = webp
            .chunk_by_id(CHUNK_VP8L)
            .and_then(|chunk| chunk.content().data())
            .map_or(false, |data| vp8l_has_alpha(data));
        if webp.has_chunk(CHUNK_ALPH) || lossless_alpha {
            flags.0[0] |= ALPHA;
        }
        if webp.has_chunk(CHUNK_EXIF) {
            flags.0[0] |= EXIF;
        }
        if webp.has_chunk(CHUNK_XMP) {
            flags.0[0] |= XMP;
        }
        if webp.has_chunk(CHUNK_ANIM) || webp.has_chunk(CHUNK_ANMF) {
            flags.0[0] |= ANIMATION;
        }
        flags
    }

    /// Replaces the ICC and EXIF flags in `flags`, the first byte
    /// of a VP8X chunk, with the ones from `self`
    pub(super) fn merge(self, flags: u8) -> u8 {
        (flags & !(ICC | EXIF)) | (self.0[0] & (ICC | EXIF))
    }
}

/// Checks the alpha_is_used bit of a VP8L bitstream header, which comes
/// after the 1 byte signature and the 14 bit width and height
fn vp8l_has_alpha(data: &[u8]) -> bool {
    matches!(data.get(..5), Some(&[0x2f, _, _, _, last]) if last & 0x10 != 0)
}
