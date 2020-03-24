pub trait ImageICC {
    fn icc_profile(&self) -> Option<Vec<u8>>;
}
