//! All encoding functions take `&self` and a writer and return the amount of written bytes. All decoding functions take
//! a buffer and return `Self` and the number of consumed bytes.
//!
//! # A note on `usize`
//!
//! `nachricht` internally uses 64 bit unsigned integers to signify field lengths. Rust however uses the
//! architecture-dependent `usize` for slice indexing. This means that on architectures where `usize` is smaller than
//! `u64` (32 bit i386 for instance), some valid `nachricht` messages can not be decoded since there would be no way to
//! efficiently index the containers. A `DecodeError::Length` will be raised in these instances. Likewise, on
//! architectures where `usize` is larger than `u64`, some valid Rust datastructures can not be encoded since there is
//! no way to represent them in the wire format. A `EncodeError::Length` will be raised in these instances.
//!
//! # A note on Maps
//!
//! The variant `Value::Map` uses a `Vec` of key-value pairs internally because Rust's floating point types `f32` and
//! `f64` implement neither `Ord` nor `Hash` and thus a nachricht `Value` cannot be used as a key in any of the standard
//! library maps.
//!
//! Likewise, `Value::Record` uses a `BTreeMap` instead of a `HashMap` because field names need to have a stable
//! ordering when deciding if a record with the same layout has already been encoded so that it can be reused.
//!
//! # Examples
//!
//! ```
//! use nachricht::*;
//! use std::borrow::Cow;
//! use std::collections::BTreeMap;
//!
//! let mut buf = Vec::new();
//! let value = Value::Record(BTreeMap::from([(Cow::Borrowed("key"), Value::Str(Cow::Borrowed("value")))]));
//! Encoder::encode(&value, &mut buf);
//! assert_eq!(buf, [
//!     0xa1, // Record of length 1
//!     0x63, // Symbol of length 3
//!     0x6b, // 'k'
//!     0x65, // 'e'
//!     0x79, // 'y'
//!     0x45, // Str of length 5
//!     0x76, // 'v'
//!     0x61, // 'a'
//!     0x6c, // 'l'
//!     0x75, // 'u',
//!     0x65, // 'e'
//! ]);
//! let decoded = Decoder::decode(&buf).unwrap();
//! assert_eq!(value, decoded.0);
//! assert_eq!(11, decoded.1);
//! ```

mod error;
mod header;
mod value;

pub use value::*;
pub use error::*;
pub use header::*;
