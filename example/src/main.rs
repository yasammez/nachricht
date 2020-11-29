use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq)]
struct UnitStruct;

#[derive(Serialize, Deserialize, PartialEq)]
struct NewtypeStruct(i8);

#[derive(Serialize, Deserialize, PartialEq)]
struct PlainStruct {
    a: i8,
    b: i8,
}

#[derive(Serialize, Deserialize, PartialEq)]
enum Enum {
    UnitVariant,
    NewtypeVariant(i8),
    TupleVariant(i32, i32),
    StructVariant { x: i32, y: i32 },
}

#[derive(Serialize, Deserialize, PartialEq)]
struct SerdeDataModel<'a> {
    boolean: bool,
    int_i8: i8,
    int_i16: i16,
    int_i32: i32,
    int_i64: i64,
    int_u8: u8,
    int_u16: u16,
    int_u32: u32,
    int_u64: u64,
    float_f32: f32,
    float_f64: f64,
    character: char,
    string: &'a str,
    owned_string: String,
    #[serde(with = "serde_bytes")]
    bytes: &'a [u8],
    #[serde(with = "serde_bytes")]
    owned_bytes: Vec<u8>,
    /*
    option: Option<i8>,
    unit: (),
    unit_struct: UnitStruct,
    newtype_struct: NewtypeStruct,
    seq: Vec<u64>,
    tuple: (i32, i32),
    map: HashMap<i32, String>,
    plain_struct: PlainStruct,
    an_enum: Enum,
    */
}


fn main() {
    let mut map = HashMap::new();
    map.insert(1, String::from("Eins"));
    map.insert(2, String::from("Zwei"));
    let data = SerdeDataModel {
        boolean: true,
        int_i8: 1,
        int_i16: -1,
        int_i32: 33434,
        int_i64: -1232454,
        int_u8: 17,
        int_u16: 16330,
        int_u32: 44444,
        int_u64: 1048576,
        float_f32: 1234.5678,
        float_f64: 1234.56789e17,
        character: 'a',
        string: "test",
        owned_string: "owned".to_owned(),
        bytes: &[1, 2, 3, 4],
        owned_bytes: vec![1, 2, 3, 4],
        /*
        option: Some(1),
        unit: (),
        unit_struct: UnitStruct,
        newtype_struct: NewtypeStruct(4),
        seq: vec![89, 734, 3453, 124, 0],
        tuple: (8, 888),
        map,
        plain_struct: PlainStruct { a: 12, b: 13 },
        an_enum: Enum::StructVariant { x: 77, y: 666 },
        */
    };

    let bytes = nachricht_serde::to_bytes(&data).unwrap();
    //dbg!(&bytes);

    let field = nachricht::Field::decode(&bytes).unwrap();
    dbg!(field);

    let decoded = nachricht_serde::from_bytes(&bytes).unwrap();
    dbg!(data == decoded);
}
