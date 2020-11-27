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
        match *self {
            EncodeError::Io(_) => f.write_str("Error when writing bytes"),
            EncodeError::Length(value) => f.write_str(&format!("Couldn't encode length {}: exceeds limit", value)),
        }
    }
}

impl From<std::io::Error> for EncodeError {
    fn from(e: std::io::Error) -> EncodeError {
        EncodeError::Io(e)
    }
}

#[derive(Debug)]
pub enum DecodeError {
    Eof,
    Code(u8),
    Length(u128),
    Utf8(std::str::Utf8Error),
    FixedValue(u64),
    DuplicateKey,
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
            DecodeError::Code(t) => f.write_str(&format!("Unexpected code {} while decoding lead byte", t)),
            DecodeError::Length(value) => f.write_str(&format!("Couldn't decode length: {} exceeds limit", value)),
            DecodeError::Utf8(_) => f.write_str("String slice was not valid Utf-8"),
            DecodeError::FixedValue(value) => f.write_str(&format!("Unrecognized value {} for Code 'Fixed'", value)),
            DecodeError::DuplicateKey => f.write_str("A key was followed directly by a key which is illegal"),
        }
    }
}
