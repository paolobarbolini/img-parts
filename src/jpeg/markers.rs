// Byte stuffing
pub const Z: u8 = 0x00;
// First marker byte
pub const P: u8 = 0xFF;

// Markers

// Start of Frame
pub const SOF0: u8 = 0xC0;
pub const SOF1: u8 = 0xC1;
pub const SOF2: u8 = 0xC2;
pub const SOF3: u8 = 0xC3;
pub const DHT: u8 = 0xC4; // Define Huffman Table
pub const SOF5: u8 = 0xC5;
pub const SOF6: u8 = 0xC6;
pub const SOF7: u8 = 0xC7;
pub const JPG: u8 = 0xC8; // JPEG Extensions
pub const SOF9: u8 = 0xC9;
pub const SOF10: u8 = 0xCA;
pub const SOF11: u8 = 0xCB;
pub const DAC: u8 = 0xCC; // Define Arithmetic Coding
pub const SOF13: u8 = 0xCD;
pub const SOF14: u8 = 0xCE;
pub const SOF15: u8 = 0xCF;

// Restart Markers
pub const RST0: u8 = 0xD0;
pub const RST1: u8 = 0xD1;
pub const RST2: u8 = 0xD2;
pub const RST3: u8 = 0xD3;
pub const RST4: u8 = 0xD4;
pub const RST5: u8 = 0xD5;
pub const RST6: u8 = 0xD6;
pub const RST7: u8 = 0xD7;

// {Start,End} of Image
pub const SOI: u8 = 0xD8;
pub const EOI: u8 = 0xD9;

// Start of Scan
pub const SOS: u8 = 0xDA;
// Define Quantization Table
pub const DQT: u8 = 0xDB;
// Define Number of Lines
pub const DNL: u8 = 0xDC;
// Define Restart Interval
pub const DRI: u8 = 0xDD;
// Define Hiercarchical Progression
pub const DHP: u8 = 0xDE;
// Expand Reference Component
pub const EXP: u8 = 0xDF;

// Application Segments
pub const APP0: u8 = 0xE0;
pub const APP1: u8 = 0xE1;
pub const APP2: u8 = 0xE2;
pub const APP3: u8 = 0xE3;
pub const APP4: u8 = 0xE4;
pub const APP5: u8 = 0xE5;
pub const APP6: u8 = 0xE6;
pub const APP7: u8 = 0xE7;
pub const APP8: u8 = 0xE8;
pub const APP9: u8 = 0xE9;
pub const APP10: u8 = 0xEA;
pub const APP11: u8 = 0xEB;
pub const APP12: u8 = 0xEC;
pub const APP13: u8 = 0xED;
pub const APP14: u8 = 0xEE;
pub const APP15: u8 = 0xEF;

// JPEG Extensions
pub const JPG0: u8 = 0xF0;
pub const JPG1: u8 = 0xF1;
pub const JPG2: u8 = 0xF2;
pub const JPG3: u8 = 0xF3;
pub const JPG4: u8 = 0xF4;
pub const JPG5: u8 = 0xF5;
pub const JPG6: u8 = 0xF6;
pub const JPG7: u8 = 0xF7;
pub const JPG8: u8 = 0xF8;
pub const JPG9: u8 = 0xF9;
pub const JPG10: u8 = 0xFA;
pub const JPG11: u8 = 0xFB;
pub const JPG12: u8 = 0xFC;
pub const JPG13: u8 = 0xFD;

// Comment
pub const COM: u8 = 0xFE;

pub(crate) fn has_length(marker: u8) -> bool {
    matches!(marker, RST0..=RST7 | APP0..=APP15 | SOF0..=SOF15 | SOS | COM | DQT | DRI)
}

pub(crate) fn has_entropy(marker: u8) -> bool {
    matches!(marker, SOS)
}
