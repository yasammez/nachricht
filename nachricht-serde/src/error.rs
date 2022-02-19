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
    Int,
    Utf8(Utf8Error),
    // Encode
    Length,
    Encode(EncodeError),
    UnknownStructLayout(&'static str),
    UnknownVariantLayout(&'static str, &'static str),
    // Preser
    DuplicateLayout(&'static str, Option<&'static str>),
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
            Error::Length => fmt.write_str("Length required"),
            Error::Trailing => fmt.write_str("Trailing characters in input"),
            Error::UnexpectedHeader(expected, actual) => write!(fmt, "Unexpected header: expected one of ({}), found {}", expected.join(", "), actual),
            Error::Utf8(e) => write!(fmt, "Bytes aren't valid Utf-8: {}", e.to_string()),
            Error::Int => fmt.write_str("Integer didn't fit into target type"),
            Error::UnknownStructLayout(l) => write!(fmt, "Layout for struct `{}` is unknown", l),
            Error::UnknownVariantLayout(l, m) => write!(fmt, "Layout for variant`{}::{}` is unknown", l, m),
            Error::DuplicateLayout(l, m) => write!(fmt, "Duplicate layout for name `{}{}`: conditionally skipping fields is not supported", l, match m { Some(x) => format!("::{}", x), None => "".into() }),
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
