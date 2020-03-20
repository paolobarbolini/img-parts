use byteorder::{ByteOrder, LittleEndian};

#[derive(Debug, Clone, PartialEq)]
pub enum VP8Kind {
    VP8,
    VP8L,
    VP8X,
}

impl VP8Kind {
    pub fn to_bytes(&self) -> [u8; 4] {
        match self {
            VP8Kind::VP8 => *b"VP8 ",
            VP8Kind::VP8L => *b"VP8L",
            VP8Kind::VP8X => *b"VP8X",
        }
    }

    pub fn from_bytes(b: &[u8]) -> Option<VP8Kind> {
        if b.len() < 4 || &b[0..3] != b"VP8" {
            return None;
        }

        match b[3] {
            b' ' => Some(VP8Kind::VP8),
            b'L' => Some(VP8Kind::VP8L),
            b'X' => Some(VP8Kind::VP8X),
            _ => None,
        }
    }
}

// the first 10 bytes are necessary
pub fn decode_size_vp8_from_header(b: &[u8]) -> (u16, u16) {
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
