use bytes::Bytes;

use crate::encoder::{EncodeAt, ImageEncoder};
use crate::jpeg::{is_jpeg, Jpeg};
use crate::png::{is_png, Png};
use crate::webp::{is_webp, WebP};
use crate::{ImageEXIF, ImageICC, Result};

/// An enum wrapping the common functions shared by the different image formats
pub enum DynImage {
    Jpeg(Jpeg),
    Png(Png),
    WebP(WebP),
}

#[allow(clippy::len_without_is_empty)]
impl DynImage {
    /// Tries to infer the file type from the file signature and calls
    /// the `from_bytes` method for the inferred format
    ///
    /// Returns `Ok(None)` if the format isn't supported.
    ///
    /// # Errors
    ///
    /// This method fails if the file is corrupted or truncated.
    pub fn from_bytes(b: Bytes) -> Result<Option<DynImage>> {
        if is_jpeg(&b) {
            let jpeg = Jpeg::from_bytes(b)?;
            Ok(Some(jpeg.into()))
        } else if is_png(&b) {
            let png = Png::from_bytes(b)?;
            Ok(Some(png.into()))
        } else if is_webp(&b) {
            let webp = WebP::from_bytes(b)?;
            Ok(Some(webp.into()))
        } else {
            Ok(None)
        }
    }

    /// Get the total size of the inner image once it is encoded
    pub fn len(&self) -> usize {
        match self {
            Self::Jpeg(jpeg) => jpeg.len(),
            Self::Png(png) => png.len(),
            Self::WebP(webp) => webp.len() as usize,
        }
    }

    /// Create an [encoder][crate::ImageEncoder] for the inner image
    #[inline]
    pub fn encoder(self) -> ImageEncoder<Self> {
        ImageEncoder::from(self)
    }
}

impl EncodeAt for DynImage {
    fn encode_at(&self, pos: &mut usize) -> Option<Bytes> {
        match self {
            Self::Jpeg(jpeg) => jpeg.encode_at(pos),
            Self::Png(png) => png.encode_at(pos),
            Self::WebP(webp) => webp.inner().encode_at(pos),
        }
    }
}

impl ImageICC for DynImage {
    fn icc_profile(&self) -> Option<Bytes> {
        match self {
            Self::Jpeg(jpeg) => jpeg.icc_profile(),
            Self::Png(png) => png.icc_profile(),
            Self::WebP(webp) => webp.icc_profile(),
        }
    }

    fn set_icc_profile(&mut self, profile: Option<Bytes>) {
        match self {
            Self::Jpeg(jpeg) => jpeg.set_icc_profile(profile),
            Self::Png(png) => png.set_icc_profile(profile),
            Self::WebP(webp) => webp.set_icc_profile(profile),
        }
    }
}

impl ImageEXIF for DynImage {
    fn exif(&self) -> Option<Bytes> {
        match self {
            Self::Jpeg(jpeg) => jpeg.exif(),
            Self::Png(png) => png.exif(),
            Self::WebP(webp) => webp.exif(),
        }
    }

    fn set_exif(&mut self, exif: Option<Bytes>) {
        match self {
            Self::Jpeg(jpeg) => jpeg.set_exif(exif),
            Self::Png(png) => png.set_exif(exif),
            Self::WebP(webp) => webp.set_exif(exif),
        }
    }
}

impl From<Jpeg> for DynImage {
    #[inline]
    fn from(jpeg: Jpeg) -> DynImage {
        DynImage::Jpeg(jpeg)
    }
}

impl From<Png> for DynImage {
    #[inline]
    fn from(png: Png) -> DynImage {
        DynImage::Png(png)
    }
}

impl From<WebP> for DynImage {
    #[inline]
    fn from(webp: WebP) -> DynImage {
        DynImage::WebP(webp)
    }
}
