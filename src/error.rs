use core::fmt;
#[cfg(feature = "std")]
use std::error::Error as StdError;

pub type Result<T> = core::result::Result<T, Error>;

/// The Errors that may occur when processing an image.
#[derive(Debug, Clone, PartialEq, Eq)]
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

#[cfg(feature = "std")]
impl StdError for Error {}
