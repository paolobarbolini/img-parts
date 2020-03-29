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
