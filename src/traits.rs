use bytes::Bytes;

/// Trait to read and write the raw ICC Profile for an image
pub trait ImageICC {
    /// Get the raw ICC Profile of this image
    ///
    /// PNG ICC profiles bigger than 16 MiB once decompressed are ignored,
    /// to limit the memory needed to read them.
    fn icc_profile(&self) -> Option<Bytes>;

    /// Overwrites the pre-existing ICC Profile of this image.
    ///
    /// Removes any pre-existing ICC Profile from this image.
    /// Adds a new ICC Profile if `profile` is `Some`.
    ///
    /// JPEG images can't store ICC profiles bigger than 16707345 bytes,
    /// so bigger ones are discarded when set on them.
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
    ///
    /// JPEG images can't store more than 65527 bytes of EXIF metadata,
    /// so bigger metadata is discarded when setting it on them.
    fn set_exif(&mut self, exif: Option<Bytes>);
}
