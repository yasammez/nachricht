use std::fmt::{Display, Formatter, self};

#[derive(Debug, PartialEq)]
pub struct DecoderError {
    inner: DecodeError,
    at: usize,
}

impl DecoderError {
    pub fn into_inner(self) -> DecodeError {
        self.inner
    }
}

impl std::error::Error for DecoderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
       Some(&self.inner)
    }
}

impl Display for DecoderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{} at input position {}", self.inner, self.at)
    }
}

#[derive(Debug, PartialEq)]
pub enum DecodeError {
    Eof,
    Utf8(std::str::Utf8Error),
    DuplicateKey(String),
    UnknownRef(usize),
    Length(u64),
    Allocation,
}

impl DecodeError {
    pub fn at(self, at: usize) -> DecoderError {
        DecoderError { inner: self, at }
    }
}

impl From<std::str::Utf8Error> for DecodeError {
    fn from(e: std::str::Utf8Error) -> DecodeError {
        DecodeError::Utf8(e)
    }
}

impl From<std::collections::TryReserveError> for DecodeError {
    fn from(_e: std::collections::TryReserveError) -> DecodeError {
        DecodeError::Allocation
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
            DecodeError::Utf8(e) => write!(f, "String slice was not valid Utf-8: {}", e),
            DecodeError::DuplicateKey(key) => write!(f, "Key {} found in value position", key),
            DecodeError::UnknownRef(value) => write!(f, "Unknown reference {}", value),
            DecodeError::Length(value) => write!(f, "Length {} exceeds maximum {}", value, usize::MAX),
            DecodeError::Allocation => f.write_str("An allocation failed"),
        }
    }
}

#[derive(Debug)]
pub enum EncodeError {
    Io(std::io::Error),
    Length(usize),
}

impl From<std::io::Error> for EncodeError {
    fn from(e: std::io::Error) -> EncodeError {
        EncodeError::Io(e)
    }
}

impl std::error::Error for EncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EncodeError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for EncodeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            EncodeError::Io(e) => write!(f, "IO error {}", e),
            EncodeError::Length(value) => write!(f, "Length {} exceeds maximum {}", value, u64::MAX),
        }
    }
}
