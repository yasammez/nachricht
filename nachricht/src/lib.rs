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
//! # Examples
//!
//! ```
//! use nachricht::*;
//!
//! let mut buf = Vec::new();
//! let field = Field { name: Some("key"), value: Value::Str("value") };
//! Encoder::encode(&field, &mut buf);
//! assert_eq!(buf, [
//!     0xc3, // Key of length 3
//!     0x6b, // 'k'
//!     0x65, // 'e'
//!     0x79, // 'y'
//!     0x85, // Str of length 5
//!     0x76, // 'v'
//!     0x61, // 'a'
//!     0x6c, // 'l'
//!     0x75, // 'u',
//!     0x65, // 'e'
//! ]);
//! let decoded = Decoder::decode(&buf).unwrap();
//! assert_eq!(field, decoded.0);
//! assert_eq!(10, decoded.1);
//! ```

mod error;
mod header;
mod field;

pub use field::*;
pub use error::*;
pub use header::*;
