use std::{fmt, io};

pub type Result<T> = std::result::Result<T, Error>;

/// The Errors that may occur when processing an image.
#[derive(Debug)]
pub enum Error {
    /// An error occurred while interacting with a Reader or a Writer
    Io(io::Error),

    /// The first two bytes of the `Jpeg` weren't a SOI marker
    FirstTwoBytesNotSOI,

    /// The first chunk of a RIFF had an id different from "RIFF"
    NoRiffHeader,

    /// The first chunk in the contents of a RIFF had an id different from "WEBP"
    NoWebpCC,
    /// A chunk of id `id` wasn't found.
    NoChunk([u8; 4]),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref err) => err.fmt(f),

            Error::FirstTwoBytesNotSOI => write!(f, "first two bytes is not a SOI marker"),

            Error::NoRiffHeader => write!(f, "no riff header"),
            Error::NoWebpCC => write!(f, "no webp cc"),
            Error::NoChunk(id) => write!(f, "no chunk of id: {:X?}", id),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}
