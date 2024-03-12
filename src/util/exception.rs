use std::{
    backtrace::Backtrace,
    error::Error,
    fmt::{self, Debug, Display, Formatter},
};

pub struct Exception {
    message: String,
    trace: String,
}

impl Exception {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            trace: Backtrace::capture().to_string(),
        }
    }
}

impl Debug for Exception {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Exception: {}\ntrace:\n{}", self.message, self.trace)
    }
}

impl Display for Exception {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for Exception {}
