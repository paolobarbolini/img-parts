/// Trait to get, add and remove the ICC Profile of an image.
pub trait ImageICC {
    /// Get the raw ICC Profile of this image
    fn icc_profile(&self) -> Option<Vec<u8>>;

    /// Overwrites the pre-existing ICC Profile of this image.
    ///
    /// Removes any pre-existing ICC Profile from this image.
    /// Adds a new ICC Profile if the `profile` is `Some`.
    fn set_icc_profile(&mut self, profile: Option<Vec<u8>>);
}

/// Trait to get, add and remove the EXIF metadata of an image.
pub trait ImageEXIF {
    /// Get the raw EXIF metadata of this image
    fn exif(&self) -> Option<Vec<u8>>;

    /// Overwrites the pre-existing EXIF metadata of this image.
    ///
    /// Removes any pre-existing EXIF metadata from this image.
    /// Adds new EXIF metadata if `exif` is `Some`.
    fn set_exif(&mut self, exif: Option<Vec<u8>>);
}
