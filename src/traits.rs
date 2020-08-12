use bytes::Bytes;

/// Trait to read and write the raw ICC Profile for an image
pub trait ImageICC {
    /// Get the raw ICC Profile of this image
    fn icc_profile(&self) -> Option<Bytes>;

    /// Overwrites the pre-existing ICC Profile of this image.
    ///
    /// Removes any pre-existing ICC Profile from this image.
    /// Adds a new ICC Profile if `profile` is `Some`.
    fn set_icc_profile(&mut self, profile: Option<Bytes>);
}

/// Trait to read and write the raw EXIF metadata for an image
pub trait ImageEXIF {
    /// Get the raw EXIF metadata of this image
    fn exif(&self) -> Option<Bytes>;

    /// Overwrites the pre-existing EXIF metadata of this image.
    ///
    /// Removes any pre-existing EXIF metadata from this image.
    /// Adds new EXIF metadata if `exif` is `Some`.
    fn set_exif(&mut self, exif: Option<Bytes>);
}
