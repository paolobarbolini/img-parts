pub use self::{image::Jpeg, segment::JpegSegment};

mod image;
pub mod markers;
mod segment;

pub(crate) fn is_jpeg(buf: &[u8]) -> bool {
    buf.len() > 4 && buf[0] == markers::P && buf[1] == markers::SOI && buf[2] == markers::P
}
