use std::{fmt, io};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),

    // webp
    NoWebpMarker,
    NoVP8X,
    NoICC,
    ImageDataReached,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref err) => err.fmt(f),

            Error::NoWebpMarker => write!(f, "no webp marker"),
            Error::NoVP8X => write!(f, "no vp8x"),
            Error::NoICC => write!(f, "no icc"),
            Error::ImageDataReached => write!(f, "image data reached"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}
