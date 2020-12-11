use std;
use std::fmt::{self, Display};
use std::str::Utf8Error;
use serde::{de, ser};
use nachricht::{EncodeError, DecodeError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Message(String),
    Encode(EncodeError),
    Decode(DecodeError),
    Length,
    Trailing,
    Unexpected,
    Utf8(Utf8Error),
    Int,
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
            Error::Decode(e) => write!(fmt, "Decoding error: {}", e.to_string()),
            Error::Length => fmt.write_str("Length required"),
            Error::Trailing => fmt.write_str("Trailing characters in input"),
            Error::Unexpected => fmt.write_str("Unexpected type encountered"),
            Error::Utf8(e) => write!(fmt, "Bytes aren't valid Utf-8: {}", e.to_string()),
            Error::Int => fmt.write_str("Integer didn't fit into a byte"),
        }
    }
}

impl From<EncodeError> for Error {
    fn from(e: EncodeError) -> Error {
        Error::Encode(e)
    }
}

impl From<DecodeError> for Error {
    fn from(e: DecodeError) -> Error {
        Error::Decode(e)
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(e: std::num::TryFromIntError) -> Error {
        Error::Int
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(e: std::str::Utf8Error) -> Error {
        Error::Utf8(e)
    }
}

impl std::error::Error for Error {}
