use crate::header::{Header, Code};
use crate::error::*;
use std::mem::size_of;
use std::io::Write;
use std::convert::TryInto;
use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub struct Field<'a> {
    name: Option<&'a str>,
    value: Value<'a>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Sign { Pos, Neg }

#[derive(Debug, PartialEq, Clone)]
pub enum Value<'a> {
    // Code `Fixed`
    Unit,
    Bool(bool),
    F32(f32),
    F64(f64),
    // Code `Bytes`
    Bytes(&'a [u8]),
    // Codes `Intp` and `Intn`
    Int(Sign, u64),
    // Code `Str`
    Str(&'a str),
    // Code `Container`
    Container(Vec<Field<'a>>),
}

enum Decodeable<'a> {
    Val(Value<'a>),
    Key(&'a str),
}

// Constants for code `Fixed`
const UNIT:  u64 = 0;
const TRUE:  u64 = 1;
const FALSE: u64 = 2;
const F32:   u64 = 3;
const F64:   u64 = 4;

impl<'a> Field<'a> {

    pub fn encode<W: Write>(&self, w: &mut W) -> Result<usize, EncodeError> {
        let mut c = 0;
        if let Some(name) = self.name {
            c += Header(Code::Key, name.len() as u64).encode(w)?;
            w.write_all(name.as_bytes())?;
            c += name.len();
        }
        c += self.value.encode(w)?;
        Ok(c)
    }

    pub fn decode<B: ?Sized + AsRef<[u8]>>(buf: &'a B) -> Result<(Self, &'a [u8]), DecodeError> {
        let (decoded, buf) = Decodeable::decode(buf)?;
        match decoded {
            Decodeable::Val(value) => Ok((Field { name: None, value }, buf)),
            Decodeable::Key(name) => {
                let (decoded, buf) = Decodeable::decode(buf)?;
                match decoded {
                    Decodeable::Val(value) => Ok((Field { name: Some(name), value }, buf)),
                    _ => Err(DecodeError::DuplicateKey)
                }
            }
        }
    }

}

impl fmt::Display for Field<'_> {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self.name {
            Some(name) => format!("@{} ", name),
            None => "".into()
        };
        let value = match &self.value {
            Value::Unit => "null".into(),
            Value::Bool(true) => "true".into(),
            Value::Bool(false)=> "false".into(),
            Value::F32(value) => format!("{}f32", value),
            Value::F64(value) => format!("{}f64", value),
            Value::Int(s, value) => format!("{}{}", match s { Sign::Pos => "+", Sign::Neg => "-" }, value),
            Value::Str(value) => "\"".to_owned() + &format!("{}", value) + "\"",
            Value::Bytes(value) => format!("{:?}", value),
            Value::Container(inner) => format!("({})", inner.iter().map(|f| format!("{}", f)).collect::<Vec<String>>().join(",")),
        };
        write!(f, "{}{}", name, value)
    }

}

impl<'a> Value<'a> {

    pub fn encode<W: Write>(&self, w: &mut W) -> Result<usize, EncodeError> {
        match &*self {
            Value::Unit => Header(Code::Fixed, UNIT).encode(w),
            Value::Bool(value) => Header(Code::Fixed, if *value { TRUE } else { FALSE }).encode(w),
            Value::F32(value) => {
                let mut c = Header(Code::Fixed, F32).encode(w)?;
                c += Self::encode_f32(*value, w)?;
                Ok(c)
            }
            Value::F64(value) => {
                let mut c = Header(Code::Fixed, F64).encode(w)?;
                c += Self::encode_f64(*value, w)?;
                Ok(c)
            },
            Value::Bytes(data) => {
                let c = Header(Code::Bytes, data.len() as u64).encode(w)?;
                w.write_all(data)?;
                Ok(c + data.len())
            }
            Value::Int(s, value) => Header(match s { Sign::Pos => Code::Intp, Sign::Neg => Code::Intn, }, *value).encode(w),
            Value::Str(data) => {
                let c = Header(Code::Str, data.len() as u64).encode(w)?;
                w.write_all(data.as_bytes())?;
                Ok(c + data.len())
            },
            Value::Container(inner) => {
                let mut c = Header(Code::Container, inner.len() as u64).encode(w)?;
                for field in inner.iter() {
                    c += field.encode(w)?;
                }
                Ok(c)
            }
        }
    }

    fn encode_f32<W: Write>(data: f32, w: &mut W) -> Result<usize, EncodeError> {
        w.write_all(&data.to_be_bytes())?;
        Ok(size_of::<f32>())
    }

    fn encode_f64<W: Write>(data: f64, w: &mut W) -> Result<usize, EncodeError> {
        w.write_all(&data.to_be_bytes())?;
        Ok(size_of::<f64>())
    }

}

impl<'a> Decodeable<'a> {

    pub fn decode<B: ?Sized + AsRef<[u8]>>(buf: &'a B) -> Result<(Self, &'a [u8]), DecodeError> {
        let (header, mut buf) = Header::decode(buf)?;
        let index = header.1 as usize;
        match header.0 {
            Code::Intp      => Ok((Self::Val(Value::Int(Sign::Pos, header.1)), buf)),
            Code::Intn      => Ok((Self::Val(Value::Int(Sign::Neg, header.1)), buf)),
            Code::Bytes     => Ok((Self::Val(Value::Bytes(&buf[..index])), &buf[index..])),
            Code::Str       => Ok((Self::Val(Value::Str(std::str::from_utf8(&buf[..index])?)), &buf[index..])),
            Code::Key       => Ok((Self::Key(std::str::from_utf8(&buf[..index])?), &buf[index..])),
            Code::Container => {
                let mut fields = Vec::with_capacity(header.1 as usize);
                for _i in 0..header.1 {
                    dbg!(&buf);
                    let (field, tmp) = Field::decode(buf)?;
                    buf = tmp;
                    fields.push(field);
                }
                Ok((Self::Val(Value::Container(fields)), buf))
            },
            Code::Fixed     => {
                match header.1 {
                    x if x == UNIT => Ok((Self::Val(Value::Unit), buf)),
                    x if x == TRUE => Ok((Self::Val(Value::Bool(true)), buf)),
                    x if x == FALSE => Ok((Self::Val(Value::Bool(false)), buf)),
                    x if x == F32 => {
                        if buf.len() < 4 {
                            Err(DecodeError::Eof)
                        } else {
                            Ok((Self::Val(Value::F32(<f32>::from_be_bytes(buf[..4].try_into().unwrap()))), &buf[4..]))
                        }
                    },
                    x if x == F64 => {
                        if buf.len() < 8 {
                            Err(DecodeError::Eof)
                        } else {
                            Ok((Self::Val(Value::F64(<f64>::from_be_bytes(buf[..8].try_into().unwrap()))), &buf[8..]))
                        }
                    },
                    i => Err(DecodeError::FixedValue(i))
                }
            },
            Code::Reserved  => Err(DecodeError::Code(header.0 as u8)),
        }
    }

}

#[cfg(test)]
mod test {
    use super::{Field, Value, Sign};

    #[test]
    fn simple_unnamed_fields() {
        let mut buf = Vec::new();
        assert_roundtrip(Field { name: None, value: Value::Unit }, &mut buf);
        assert_roundtrip(Field { name: None, value: Value::Bool(true) }, &mut buf);
        assert_roundtrip(Field { name: None, value: Value::Bool(false) }, &mut buf);
        for i in (0..u64::MAX).step_by(3_203_431_780_337) {
            assert_roundtrip(Field { name: None, value: Value::Int(Sign::Pos, i) }, &mut buf);
            assert_roundtrip(Field { name: None, value: Value::Int(Sign::Neg, i) }, &mut buf);
        }
    }

    #[test]
    fn floats() {
        let mut buf = Vec::new();
        assert_roundtrip(Field { name: None, value: Value::F64(f64::MAX) }, &mut buf);
        assert_roundtrip(Field { name: None, value: Value::F64(f64::MIN) }, &mut buf);
        assert_roundtrip(Field { name: None, value: Value::F64(std::f64::consts::PI) }, &mut buf);
        assert_roundtrip(Field { name: None, value: Value::F32(f32::MAX) }, &mut buf);
        assert_roundtrip(Field { name: None, value: Value::F32(f32::MIN) }, &mut buf);
        assert_roundtrip(Field { name: None, value: Value::F32(std::f32::consts::PI) }, &mut buf);
    }

    #[test]
    fn simple_named_fields() {
        let mut buf = Vec::new();
        assert_roundtrip(Field { name: Some("null"), value: Value::Unit }, &mut buf);
        assert_roundtrip(Field { name: Some("bool"), value: Value::Bool(true) }, &mut buf);
        assert_roundtrip(Field { name: Some("bool"), value: Value::Bool(false) }, &mut buf);
        for i in (0..u64::MAX).step_by(3_203_431_780_337) {
            assert_roundtrip(Field { name: Some("integer"), value: Value::Int(Sign::Pos, i) }, &mut buf);
            assert_roundtrip(Field { name: Some("integer"), value: Value::Int(Sign::Neg, i) }, &mut buf);
        }
    }

    #[test]
    fn strings() {
        let mut buf = Vec::new();
        assert_roundtrip(Field { name: None, value: Value::Str("Hello World! This is not ascii: äöüß") }, &mut buf);
    }

    #[test]
    fn bytes() {
        let mut buf = Vec::new();
        assert_roundtrip(Field { name: None, value: Value::Bytes(&[1, 2, 3, 4, 255]) }, &mut buf);
    }

    #[test]
    fn array_mixed() {
        let mut buf = Vec::new();
        assert_roundtrip(Field { name: Some("array"), value: Value::Container(vec![
                Field { name: None, value: Value::Int(Sign::Pos, 1) },
                Field { name: None, value: Value::Int(Sign::Pos, 2) },
                Field { name: None, value: Value::Int(Sign::Pos, 3) },
                Field { name: None, value: Value::Int(Sign::Pos, 4) },
        ])}, &mut buf);
    }

    #[test]
    fn array_long() {
        let mut buf = Vec::new();
        for i in (0..1 << 10) {
            assert_roundtrip(Field { name: Some("array"), value: Value::Container(vec![
                    Field { name: None, value: Value::Int(Sign::Pos, 1) }; i as usize
            ])}, &mut buf);
        }
    }

    #[test]
    fn map() {
        let mut buf = Vec::new();
        assert_roundtrip(Field { name: Some("array"), value: Value::Container(vec![
                Field { name: Some("first"), value: Value::Int(Sign::Pos, 1) },
                Field { name: Some("second"), value: Value::Int(Sign::Pos, 2) },
                Field { name: Some("third"), value: Value::Int(Sign::Pos, 3) },
                Field { name: Some("fourth"), value: Value::Int(Sign::Pos, 4) },
        ])}, &mut buf);
    }

    #[test]
    fn display() {
        let object = Field { name: None, value: Value::Container(vec![
            Field { name: Some("integer"), value: Value::Int(Sign::Pos, 1337) },
            Field { name: Some("bool"), value: Value::Bool(true) },
            Field { name: Some("optional"), value: Value::Unit },
            Field { name: Some("array"), value: Value::Container(vec![
                Field { name: None, value: Value::Int(Sign::Pos, 8472) },
                Field { name: None, value: Value::Int(Sign::Neg, 404) },
                Field { name: None, value: Value::Str("Hallo \"Welt\"\n") },
            ])},
            Field { name: Some("bytes"), value: Value::Bytes(&[1, 2, 3, 4]) },
            Field { name: Some("float"), value: Value::F64(std::f64::consts::E) }
        ])};
        println!("{}", &object);
        let mut buf = Vec::new();
        let _ = object.encode(&mut buf);
        dbg!(buf);
        assert_eq!(format!("{}", &object), "(@integer +1337,@bool true,@null optional,@array (+8472,-404,\"Hallo \\\"Welt\\\"\\n\"),@bytes [1, 2, 3, 4],@float 2.718281828459045f64)");
    }


    fn assert_roundtrip(value: Field, buf: &mut Vec<u8>) {
        let _ = value.encode(buf);
        assert_eq!(value, Field::decode(buf).unwrap().0);
        buf.clear();
    }

}
