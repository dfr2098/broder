use std::error::Error;
use std::fmt::{self, Display};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpatialError {
    message: String,
}

impl SpatialError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for SpatialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for SpatialError {}
