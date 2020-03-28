use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct InvalidBikeNetwork {
    name: String,
}
impl Error for InvalidBikeNetwork {}

impl InvalidBikeNetwork {
    pub fn new(name: String) -> InvalidBikeNetwork {
        InvalidBikeNetwork { name }
    }
}

impl fmt::Display for InvalidBikeNetwork {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InvalidBikeNetwork name: {}", self.name)
    }
}
