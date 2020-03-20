pub use error::{Error, Result};
pub use webp::{add_icc_to_webp, icc_from_webp};

mod error;
pub(crate) mod riff;
pub(crate) mod vp8;
mod webp;
