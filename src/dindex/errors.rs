use std::fmt;

#[derive(Debug)]
pub struct DeserializationError(String);

impl DeserializationError {
    pub(super) fn new(msg: &str) -> DeserializationError {
        DeserializationError(msg.to_string())
    }
}

impl fmt::Display for DeserializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl From<&str> for DeserializationError {
    fn from(str: &str) -> DeserializationError {
        DeserializationError(String::from(str))
    }
}

impl std::error::Error for DeserializationError {}
