//! Conveniently serialize and deserialize your Rust data structures into the `nachricht` wire format.
//!
//! # Two-step serialization
//!
//! Nachricht is able to re-use record layouts. However, serde only provides the name of a struct
//! when serializing. The struct's actual layout can only be discovered by serializing it. This means
//! that for recursive data structures this information comes too late and no reuse would be possible.
//! To circumvent this, we employ a [preser::Preserializer](Preserializer) which fills
//! HashMaps which correlate the struct names with their layouts. However, if one name is used for
//! two different layouts, serialization fails and an error is reported. This situation can arise when
//! conditionally skipping fields, for instance with
//! `#[serde(skip_serializing_if = "Option::is_none")]`. This is a shortcoming of serde, not nachricht!
//!
//! # Examples
//!
//! This example demonstrates some of `nachricht`'s capabilities, including the re-use of struct
//! layouts and enum constants with the help of an internal symbol table.
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
//! // (
//! //   cats: [
//! //     (
//! //       name: "Jessica",
//! //       species: #PrionailurusViverrinus,
//! //     ),
//! //     (
//! //       name: "Wantan",
//! //       species: #LynxLynx,
//! //     ),
//! //     (
//! //       name: "Sphinx",
//! //       species: #FelisCatus,
//! //     ),
//! //     (
//! //       name: "Chandra",
//! //       species: #PrionailurusViverrinus,
//! //     ),
//! //   ],
//! //   version: 1,
//! // )
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
//! assert_eq!(bytes.len(), 107);
//! assert_eq!(bytes, [
//!   0xa2,                                                   // Record of length 2
//!     0x67,                                                 // Symbol of length 7 - this gets inserted into the symbol table at index 0
//!       0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e,           // 'version'
//!     0x64,                                                 // Symbol of length 4 - this gets inserted into the symbol table at index 1
//!       0x63, 0x61, 0x74, 0x73,                             // 'cats' - struct Message gets inserted into the symbol table at index 2
//!     0x21,                                                 // positive integer 1
//!     0x84,                                                 // Array of length 4
//!       0xa2,                                               // Record of length 2 - this is the first cat
//!         0x64,                                             // Symbol of length 4 - index 3 in symbol table
//!           0x6e, 0x61, 0x6d, 0x65,                         // 'name'
//!         0x67,                                             // Symbol of length 7 - index 4 in symbol table
//!           0x73, 0x70, 0x65, 0x63, 0x69, 0x65, 0x73,       // 'species' - struct Cat at index 5 in symbol table
//!         0x47,                                             // String of length 7 - strings do not get inserted into the symbol table
//!           0x4a, 0x65, 0x73, 0x73, 0x69, 0x63, 0x61,       // 'Jessica'
//!         0x76,                                             // Symbol of length 22 - index 5 in symbol table
//!           0x50, 0x72, 0x69, 0x6f, 0x6e, 0x61, 0x69, 0x6c, // 'PrionailurusViverrinus'
//!           0x75, 0x72, 0x75, 0x73, 0x56, 0x69, 0x76, 0x65,
//!           0x72, 0x72, 0x69, 0x6e, 0x75, 0x73,
//!       0xe5,                                               // Reference to symbols[5] which resolves to struct Cat - this is the second cat
//!         0x46,                                             // String of length 6
//!           0x57, 0x61, 0x6e, 0x74, 0x61, 0x6e,             // 'Wantan'
//!         0x68,                                             // Symbol of length 8 - index 6 in symbol table
//!           0x4c, 0x79, 0x6e, 0x78, 0x4c, 0x79, 0x6e, 0x78, // 'LynxLynx'
//!       0xe5,                                               // &symbols[5] - this is the third cat
//!         0x46,                                             // String of length 6
//!           0x53, 0x70, 0x68, 0x69, 0x6e, 0x78,             // 'Sphinx'
//!         0x6a,                                             // Symbol of length 10 - index 7 in symbol table
//!           0x46, 0x65, 0x6c, 0x69, 0x73, 0x43, 0x61, 0x74, // 'FelisCatus'
//!           0x75, 0x73,
//!       0xe5,                                               // &symbols[5] - this is the fourth and last cat
//!         0x47,                                             // String of length 7
//!           0x43, 0x68, 0x61, 0x6e, 0x64, 0x72, 0x61,       // 'Chandra'
//!         0xe6,                                             // Reference to symbols[6] which resolves to Symbol('PrionailurusViverrinus')
//! ]);
//!
//! let deserialized = nachricht_serde::from_bytes(&bytes).unwrap();
//! assert_eq!(msg, deserialized);
//!
//! ```
//!
//! Note how efficient the encoding becomes for repetetive data structures: the last `species: #PrionailurusViverrinus`
//! only requires one byte on wire, which leads to the whole message only occupying 107 bytes!
//!
//! For comparison, `serde_json` produces a string of 210 bytes for the given input while msgpack in self-describing
//! mode still needs 176 bytes. Non-self-describing formats like flatbuffers or bincode can of course achieve even
//! smaller sizes at the expense of needing prior knowledge to make sense of the message.

mod de;
mod error;
mod preser;
mod ser;

pub use de::{from_bytes, Deserializer};
pub use error::{Error, Result};
pub use ser::{to_bytes, to_writer, Serializer};

#[cfg(test)]
mod tests {
    use serde::{Serialize, Deserialize};
    use std::collections::HashMap;
    use super::{to_bytes, from_bytes};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    enum Enum {
        UnitVariant,
        NewtypeVariant(bool),
        TupleVariant(f32, f32),
        StructVariant{ a: usize, b: usize, c: usize },
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Struct {
        field: u8,
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct UnitStruct;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct NewtypeStruct(String);

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TupleStruct(char, char, char);

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Test {
        bool: bool,
        i8: i8,
        i16: i16,
        i32: i32,
        i64: i64,
        u8: u8,
        u16: u16,
        u32: u32,
        u64: u64,
        f32: f32,
        f64: f64,
        char: char,
        str: String,
        #[serde(with = "serde_bytes")]
        bytes: Vec<u8>,
        none: Option<u8>,
        some: Option<u8>,
        unit: (),
        unit_struct: UnitStruct,
        newtype_struct: NewtypeStruct,
        seq: Vec<String>,
        tuple: (u16, u16, u16),
        map: HashMap<usize, String>,
        r#struct: Struct,
        unit_variant: Enum,
        newtype_variant: Enum,
        tuple_variant: Enum,
        struct_variant: Enum,
    }

    #[test]
    fn roundtrip() {
        let message = Test {
            bool: true,
            i8: -1,
            i16: -20,
            i32: -7000,
            i64: i64::MIN,
            u8: 1,
            u16: 20,
            u32: 7000,
            u64: u64::MAX,
            f32: 1337.8472,
            f64: 1337.8472,
            char: 'x',
            str: "Test".to_string(),
            bytes: vec![
                0xa2, 0x67, 0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e, 0x64, 0x63, 0x61, 0x74, 0x73,
                0x21, 0x84, 0xa2, 0x64, 0x6e, 0x61, 0x6d, 0x65, 0x67, 0x73, 0x70, 0x65, 0x63, 0x69,
                0x65, 0x73, 0x47, 0x4a, 0x65, 0x73, 0x73, 0x69, 0x63, 0x61, 0x76, 0x50, 0x72, 0x69,
                0x6f, 0x6e, 0x61, 0x69, 0x6c, 0x75, 0x72, 0x75, 0x73, 0x56, 0x69, 0x76, 0x65, 0x72,
                0x72, 0x69, 0x6e, 0x75, 0x73, 0xe5, 0x46, 0x57, 0x61, 0x6e, 0x74, 0x61, 0x6e, 0x68,
                0x4c, 0x79, 0x6e, 0x78, 0x4c, 0x79, 0x6e, 0x78, 0xe5, 0x46, 0x53, 0x70, 0x68, 0x69,
                0x6e, 0x78, 0x6a, 0x46, 0x65, 0x6c, 0x69, 0x73, 0x43, 0x61, 0x74, 0x75, 0x73, 0xe5,
                0x47, 0x43, 0x68, 0x61, 0x6e, 0x64, 0x72, 0x61, 0xe6,
            ],
            none: None,
            some: Some(0),
            unit: (),
            unit_struct: UnitStruct,
            newtype_struct: NewtypeStruct("Qapla'".to_string()),
            seq: vec![
                "Elen".to_string(),
                "síla".to_string(),
                "lúmenn'".to_string(),
                "omentielvo".to_string(),
            ],
            tuple: (0, 0, 0),
            map: [
                (1701, "Enterprise".to_string()),
                (74656, "Voyager".to_string())
            ].into_iter().collect(),
            r#struct: Struct {
                field: 42,
            },
            unit_variant: Enum::UnitVariant,
            newtype_variant: Enum::NewtypeVariant(false),
            tuple_variant: Enum::TupleVariant(1.0, 0.999),
            struct_variant: Enum::StructVariant {
                a: 255,
                b: 0,
                c: 33,
            }
        };
        println!("{:02x?}", to_bytes(&message));
        assert_eq!(message, from_bytes::<Test>(&to_bytes(&message).unwrap()).unwrap());
    }
}