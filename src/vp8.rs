use byteorder::{ByteOrder, LittleEndian};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum VP8Kind {
    /// A WEBP image in Simple File Format (Lossy)
    VP8,
    /// A WEBP image in Simple File Format (Lossless)
    VP8L,
    /// A WEBP image in Extended File format
    VP8X,
}

// the first 10 bytes are necessary
pub(crate) fn size_from_vp8_header(b: &[u8]) -> (u16, u16) {
    let tag = LittleEndian::read_u24(&b[0..3]);

    let keyframe = tag & 1 == 0;

    if keyframe {
        if b[3..6] != [0x9d, 0x01, 0x2a] {
            panic!("invalid frame magic bytes");
        }

        let width = LittleEndian::read_u16(&b[6..8]);
        let height = LittleEndian::read_u16(&b[8..10]);

        (width & 0x3FFF, height & 0x3FFF)
    } else {
        panic!("expected keyframe")
    }
}
