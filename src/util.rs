use std::convert::TryInto;

pub fn u24_from_le_bytes(b: [u8; 3]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], 0])
}

pub fn u24_to_le_bytes(n: u32) -> [u8; 3] {
    let b = n.to_le_bytes();
    b[0..3].try_into().unwrap()
}
