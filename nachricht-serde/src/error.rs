use std;
use std::fmt::{self, Display};
use serde::{de, ser};
use nachricht::{EncodeError, DecodeError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Message(String),
    Encode(EncodeError),
    Length,
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Message(msg) => fmt.write_str(msg),
            Error::Encode(e) => write!(fmt, "Encoding error: {}", e.to_string()),
            Error::Length => fmt.write_str("Length required"),
        }
    }
}

impl From<EncodeError> for Error {
    fn from(e: EncodeError) -> Error {
        Error::Encode(e)
    }
}

impl std::error::Error for Error {}
