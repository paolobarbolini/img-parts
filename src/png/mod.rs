pub use self::{chunk::PngChunk, image::Png};

mod chunk;
mod image;

pub(crate) fn is_png(buf: &[u8]) -> bool {
    buf.starts_with(image::SIGNATURE)
}
