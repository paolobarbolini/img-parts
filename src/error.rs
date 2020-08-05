use std::{fmt, io};

pub type Result<T> = std::result::Result<T, Error>;

/// The Errors that may occur when processing an image.
#[derive(Debug)]
pub enum Error {
    /// An error occurred while interacting with a Reader or a Writer
    Io(io::Error),

    /// The file signature didn't match the expected signature
    WrongSignature,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref err) => err.fmt(f),

            Error::WrongSignature => {
                write!(f, "the file signature didn't match the expected signature")
            }
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}
