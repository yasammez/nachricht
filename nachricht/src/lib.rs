//! All encoding functions take `&self` and a writer and return the amount of bytes written. All
//! decoding functions take a buffer and return `Self` and the unprocessed tail of the buffer.

mod error;
mod header;
mod field;

#[doc(hidden)]
pub use field::*;

#[doc(hidden)]
pub use error::*;

#[doc(hidden)]
pub use header::*;
