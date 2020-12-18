use std::fmt::{Display, Formatter, self};

#[derive(Debug)]
pub enum EncodeError {
    Io(std::io::Error),
    Length(u128),
}

impl std::error::Error for EncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EncodeError::Io(e) => Some(e),
            EncodeError::Length(_) => None,
        }
    }
}

impl Display for EncodeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            EncodeError::Io(e) => write!(f, "IO Error when writing bytes: {}", e),
            EncodeError::Length(value) => write!(f, "Couldn't encode length {}: exceeds limit", value),
        }
    }
}

impl From<std::io::Error> for EncodeError {
    fn from(e: std::io::Error) -> EncodeError {
        EncodeError::Io(e)
    }
}

#[derive(Debug, PartialEq)]
pub enum DecodeError {
    Eof,
    Code(u8),
    Length(u128),
    Utf8(std::str::Utf8Error),
    DuplicateKey,
    UnknownRef(u64),
}

impl From<std::str::Utf8Error> for DecodeError {
    fn from(e: std::str::Utf8Error) -> DecodeError {
        DecodeError::Utf8(e)
    }
}

impl std::error::Error for DecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DecodeError::Utf8(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for DecodeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            DecodeError::Eof => f.write_str("Unexpected end of buffer while decoding"),
            DecodeError::Code(t) => write!(f, "Unexpected code {} while decoding header", t),
            DecodeError::Length(value) => write!(f, "Couldn't decode length: {} exceeds limit", value),
            DecodeError::Utf8(e) => write!(f, "String slice was not valid Utf-8: {}", e),
            DecodeError::DuplicateKey => f.write_str("A key was followed directly by a key which is illegal"),
            DecodeError::UnknownRef(value) => write!(f, "Unknown reference {}", value),
        }
    }
}
