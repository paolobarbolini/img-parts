use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

/// The Errors that may occur when processing an image.
#[derive(Debug)]
pub enum Error {
    /// The file signature didn't match the expected signature
    WrongSignature,

    /// The chunk CRC didn't match the expected calculated CRC
    BadCRC,

    /// A truncated chunk was read
    Truncated,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::WrongSignature => {
                f.write_str("the file signature didn't match the expected signature")
            }
            Self::BadCRC => f.write_str("the chunk CRC didn't match the expected calculated CRC"),
            Self::Truncated => f.write_str("a truncated chunk was read"),
        }
    }
}
