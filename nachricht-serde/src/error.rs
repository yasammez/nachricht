use std;
use std::fmt::{self, Display};
use std::str::Utf8Error;
use serde::{de, ser};
use nachricht::{EncodeError, DecodeError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct DeserializationError {
    inner: Error,
    at: usize,
}

impl std::error::Error for DeserializationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.inner)
    }
}

impl Display for DeserializationError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{} at input position {}", self.inner, self.at)
    }
}

#[derive(Debug)]
pub enum Error {
    // Decode
    Decode(DecodeError),
    Trailing,
    UnexpectedHeader(&'static [&'static str], &'static str),
    UnexpectedRefable(&'static str, &'static str),
    Int,
    Utf8(Utf8Error),
    Key(String, &'static str),
    // Encode
    Length,
    Encode(EncodeError),
    KeyType,
    // Both
    Message(String),
}

impl Error {
    pub fn at(self, at: usize) -> DeserializationError {
        DeserializationError { inner: self, at }
    }
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
            Error::KeyType => write!(fmt, "Map key must be convertible to a string. Maybe use crate `serde_with` to transform the map into a vec of tuples"),
            Error::Length => fmt.write_str("Length required"),
            Error::Trailing => fmt.write_str("Trailing characters in input"),
            Error::UnexpectedHeader(expected, actual) => write!(fmt, "Unexpected header: expected one of ({}), found {}", expected.join(", "), actual),
            Error::UnexpectedRefable(expected, actual) => write!(fmt, "Unexpected refable: expected {}, found {}", expected, actual),
            Error::Utf8(e) => write!(fmt, "Bytes aren't valid Utf-8: {}", e.to_string()),
            Error::Key(k, t) => write!(fmt, "Key `{}` could not be parsed as {}", k, t),
            Error::Int => fmt.write_str("Integer didn't fit into target type"),
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
    fn from(_e: std::num::TryFromIntError) -> Error {
        Error::Int
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(e: std::str::Utf8Error) -> Error {
        Error::Utf8(e)
    }
}

impl std::error::Error for Error {}
