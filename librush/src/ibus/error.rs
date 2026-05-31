//! 错误信息

use std::error::Error;
use std::fmt::{Display, Formatter};

/// Custom error info for this crate
#[derive(Debug, Clone, PartialEq)]
pub struct IBusErr {
    msg: String,
}

impl IBusErr {
    pub fn new(msg: String) -> Self {
        IBusErr { msg }
    }
}

impl Error for IBusErr {}

impl Display for IBusErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}
