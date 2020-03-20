pub use error::{Error, Result};
pub use riff::RiffChunk;
pub use vp8::VP8Kind;
pub use webp::{WebP, WebPFlags};

mod error;
mod riff;
pub(crate) mod vp8;
mod webp;
