//! Conveniently serialize and deserialize your Rust data structures into the `nachricht` wire format.
//!
//! # Examples
//!
//! This example demonstrates some of `nachricht`'s capabilities, including the re-use of field identifiers and enum
//! constants with the help of an internal symbol table.
//!
//! ```
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize, PartialEq, Debug)]
//! pub enum Species {
//!     PrionailurusViverrinus,
//!     LynxLynx,
//!     FelisCatus,
//! }
//!
//! #[derive(Serialize, Deserialize, PartialEq, Debug)]
//! pub struct Cat<'a> {
//!     name: &'a str,
//!     species: Species,
//! }
//!
//! #[derive(Serialize, Deserialize, PartialEq, Debug)]
//! struct Message<'a> {
//!     version: u32,
//!     #[serde(borrow)]
//!     cats: Vec<Cat<'a>>,
//! }
//!
//! // This message has the following canonical `nachricht` string representation as ´nq` would
//! // produce.
//! // ```
//! //(
//! //  version = 1,
//! //  cats = (
//! //   (
//! //     name = "Jessica",
//! //     species = #PrionailurusViverrinus,
//! //   ),
//! //   (
//! //     name = "Wantan",
//! //     species = #LynxLynx,
//! //   ),
//! //   (
//! //     name = "Sphinx",
//! //     species = #FelisCatus,
//! //   ),
//! //   (
//! //     name = "Chandra",
//! //     species = #PrionailurusViverrinus,
//! //   ),
//! // ),
//! //)
//! // ```
//! let msg = Message {
//!     version: 1,
//!     cats: vec![
//!         Cat { name: "Jessica", species: Species::PrionailurusViverrinus },
//!         Cat { name: "Wantan", species: Species::LynxLynx },
//!         Cat { name: "Sphinx", species: Species::FelisCatus },
//!         Cat { name: "Chandra", species: Species::PrionailurusViverrinus },
//!     ],
//! };
//!
//! let bytes = nachricht_serde::to_bytes(&msg).unwrap();
//! assert_eq!(bytes.len(), 113);
//! assert_eq!(bytes, [
//!   0x62,                                                   // Container of length 2
//!     0xc7,                                                 // Key of length 7 - this gets inserted into the symbol table at index 0
//!       0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e,           // 'version'
//!     0x21,                                                 // positive integer 1
//!     0xc4,                                                 // Key of length 4 - this gets inserted into the symbol table at index 1
//!       0x63, 0x61, 0x74, 0x73,                             // 'cats'
//!     0x64,                                                 // Container of length 4
//!       0x62,                                               // Container of length 2 - this is the first cat
//!         0xc4,                                             // Key of length 4 - index 2 in symbol table
//!           0x6e, 0x61, 0x6d, 0x65,                         // 'name'
//!         0x87,                                             // String of length 7 - strings do not get inserted into the symbol table
//!           0x4a, 0x65, 0x73, 0x73, 0x69, 0x63, 0x61,       // 'Jessica'
//!         0xc7,                                             // Key of length 7 - index 3 in symbol table
//!           0x73, 0x70, 0x65, 0x63, 0x69, 0x65, 0x73,       // 'species'
//!         0xb6,                                             // Symbol of length 22 - index 4 in symbol table
//!           0x50, 0x72, 0x69, 0x6f, 0x6e, 0x61, 0x69, 0x6c, // 'PrionailurusViverrinus'
//!           0x75, 0x72, 0x75, 0x73, 0x56, 0x69, 0x76, 0x65,
//!           0x72, 0x72, 0x69, 0x6e, 0x75, 0x73,
//!       0x62,                                               // Container of length 2 - this is the second cat
//!         0xe2,                                             // Reference to symbols[2] which resolves to Key('name')
//!         0x86,                                             // String of length 6
//!           0x57, 0x61, 0x6e, 0x74, 0x61, 0x6e,             // 'Wantan'
//!         0xe3,                                             // Reference to symbols[3] which resolves to Key('species')
//!         0xa8,                                             // Symbol of length 8 - index 5 in symbol table
//!           0x4c, 0x79, 0x6e, 0x78, 0x4c, 0x79, 0x6e, 0x78, // 'LynxLynx'
//!       0x62,                                               // Container of length 3 - this is the third cat
//!         0xe2,                                             // Reference to symbols[2] which resolves to Key('name')
//!         0x86,                                             // String of length 6
//!           0x53, 0x70, 0x68, 0x69, 0x6e, 0x78,             // 'Sphinx'
//!         0xe3,                                             // Reference to symbols[3] which resolves to Key('species')
//!         0xaa,                                             // Symbol of length 10 - index 6 in symbol table
//!           0x46, 0x65, 0x6c, 0x69, 0x73, 0x43, 0x61, 0x74, // 'FelisCatus'
//!           0x75, 0x73,
//!       0x62,                                               // Container of length 3 - this is the fourth and last cat
//!         0xe2,                                             // Reference to symbols[2] which resolves to Key('name')
//!         0x87,                                             // String of length 7
//!           0x43, 0x68, 0x61, 0x6e, 0x64, 0x72, 0x61,       // 'Chandra'
//!         0xe3,                                             // Reference to symbols[3] which resolves to Key('species')
//!         0xe4,                                             // Reference to symbols[4] which resolves to Symbol('PrionailurusViverrinus')
//! ]);
//!
//! let deserialized = nachricht_serde::from_bytes(&bytes).unwrap();
//! assert_eq!(msg, deserialized);
//!
//! ```
//!
//! Note how efficient the encoding becomes for repetetive data structures: the last `species = #PrionailurusViverrinus`
//! only requires two bytes on wire, which leads to the whole message only occupying 113 bytes!
//!
//! For comparison, `serde_json` produces a string of 210 bytes for the given input while msgpack in self-describing
//! mode still needs 176 bytes. Non-self-describing formats like flatbuffers or bincode can of course achieve even
//! smaller sizes at the expense of needing prior knowledge to make sense of the message.


mod de;
mod error;
mod ser;

pub use de::{from_bytes, Deserializer};
pub use error::{Error, Result};
pub use ser::{to_bytes, Serializer};
