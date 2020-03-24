pub trait ImageICC {
    fn icc_profile(&self) -> Option<Vec<u8>>;
    fn set_icc_profile(&mut self, profile: Option<Vec<u8>>);
}
