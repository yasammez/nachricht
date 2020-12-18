use crate::header::Header;
use crate::error::{DecodeError, DecoderError, EncodeError};
use std::mem::size_of;
use std::io::Write;
use std::convert::TryInto;
use std::str::from_utf8;

#[derive(Debug, PartialEq, Clone)]
pub enum Sign { Pos, Neg }

#[derive(Debug, PartialEq, Clone)]
pub enum Value<'a> {
    Null,
    Bool(bool),
    F32(f32),
    F64(f64),
    Bytes(&'a [u8]),
    Int(Sign, u64),
    Str(&'a str),
    Symbol(&'a str),
    Container(Vec<Field<'a>>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Field<'a> {
    pub name: Option<&'a str>,
    pub value: Value<'a>,
}

#[derive(PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum Refable {
    Sym,
    Key,
}

pub struct Encoder<'w, W: Write> {
    writer: &'w mut W,
    symbols: Vec<(Refable, String)>,
}

impl<'w, W: Write> Encoder<'w, W> {

    pub fn encode(field: &Field, writer: &'w mut W) -> Result<usize, EncodeError> {
        Self { writer, symbols: Vec::new() }.encode_inner(field)
    }

    fn encode_inner(&mut self, field: &Field) -> Result<usize, EncodeError> {
        let mut c = 0;
        if let Some(name) = field.name {
            c += self.encode_refable(name, Refable::Key)?;
        }
        match &field.value {
            Value::Null      => Header::Null.encode(self.writer),
            Value::Bool(v)   => match v { true => Header::True, false => Header::False }.encode(self.writer),
            Value::F32(v)    => {
                c += Header::F32.encode(self.writer)?;
                self.writer.write_all(&v.to_be_bytes())?;
                Ok(c + size_of::<f32>())
            },
            Value::F64(v)    => {
                c += Header::F64.encode(self.writer)?;
                self.writer.write_all(&v.to_be_bytes())?;
                Ok(c + size_of::<f64>())
            },
            Value::Bytes(v)  => {
                c += Header::Bin(v.len()).encode(self.writer)?;
                self.writer.write_all(v)?;
                Ok(c + v.len())
            },
            Value::Int(s, v) => { match s { Sign::Pos => Header::Pos(*v), Sign::Neg => Header::Neg(*v) }.encode(self.writer) },
            Value::Str(v) => {
                c += Header::Str(v.len()).encode(self.writer)?;
                self.writer.write_all(v.as_bytes())?;
                Ok(c + v.len())
            },
            Value::Symbol(v) => self.encode_refable(v, Refable::Sym),
            Value::Container(inner) => {
                c += Header::Bag(inner.len()).encode(self.writer)?;
                for field in inner.iter() {
                    c += self.encode_inner(field)?;
                }
                Ok(c)
            },
        }
    }

    fn encode_refable<'a>(&mut self, key: &'a str, kind: Refable) -> Result<usize, EncodeError> {
        match self.symbols.iter().enumerate().find(|(_, (k, v))| *k == kind && v == key) {
            Some((i, _)) => Header::Ref(i).encode(self.writer),
            None         => {
                self.symbols.push((kind, key.to_owned()));
                match kind { Refable::Key => Header::Key(key.len()), Refable::Sym => Header::Sym(key.len()) }.encode(self.writer)?;
                self.writer.write_all(key.as_bytes())?;
                Ok(1 + key.len())
            }
        }
    }

}

pub struct Decoder<'a> {
    symbols: Vec<(Refable, &'a str)>,
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Decoder<'a> {

    pub fn decode<B: ?Sized + AsRef<[u8]>>(buf: &'a B) -> Result<(Field<'a>, usize), DecoderError> {
        let mut decoder = Self { buf: buf.as_ref(), symbols: Vec::new(), pos: 0 };
        let field = decoder.decode_field().map_err(|e| e.at(decoder.pos))?;
        Ok((field, decoder.pos))
    }

    fn decode_field(&mut self) -> Result<Field<'a>, DecodeError> {
        let (header, c) = Header::decode(&self.buf[self.pos..])?;
        self.pos += c;
        match header {
            Header::Key(v) => {
                let key = from_utf8(&self.decode_slice(v)?)?;
                self.symbols.push((Refable::Key, key));
                Ok(Field { name: Some(key), value: self.decode_value()? })
            },
            Header::Ref(v) => {
                match self.symbols.get(v) {
                    Some((Refable::Sym, _)) => Ok(Field { name: None, value: self.decode_value_inner(header)? }),
                    Some((Refable::Key, key)) => Ok(Field { name: Some(key), value: self.decode_value()? }),
                    _ => Err(DecodeError::UnknownRef(v)),
                }
            },
            _ => Ok(Field { name: None, value: self.decode_value_inner(header)? }),
        }
    }

    fn decode_value(&mut self) -> Result<Value<'a>, DecodeError> {
        let (header, c) = Header::decode(&self.buf[self.pos..])?;
        self.pos += c;
        self.decode_value_inner(header)
    }

    fn decode_value_inner(&mut self, header: Header) -> Result<Value<'a>, DecodeError> {
        match header {
            Header::Null   => Ok(Value::Null),
            Header::True   => Ok(Value::Bool(true)),
            Header::False  => Ok(Value::Bool(false)),
            Header::F32    => Ok(Value::F32(<f32>::from_be_bytes(self.decode_slice(4)?.try_into().unwrap()))),
            Header::F64    => Ok(Value::F64(<f64>::from_be_bytes(self.decode_slice(8)?.try_into().unwrap()))),
            Header::Bin(v) => Ok(Value::Bytes(self.decode_slice(v)?)),
            Header::Pos(v) => Ok(Value::Int(Sign::Pos, v)),
            Header::Neg(v) => Ok(Value::Int(Sign::Neg, v)),
            Header::Bag(v) => {
                let mut fields = Vec::with_capacity(v);
                for _ in 0..v {
                    fields.push(self.decode_field()?);
                }
                Ok(Value::Container(fields))
            },
            Header::Str(v) => Ok(Value::Str(from_utf8(&self.decode_slice(v)?)?)),
            Header::Sym(v) => {
                let symbol = from_utf8(&self.decode_slice(v)?)?;
                self.symbols.push((Refable::Sym, symbol));
                Ok(Value::Symbol(symbol))
            },
            Header::Key(v) => {
                let key = from_utf8(&self.decode_slice(v)?)?;
                Err(DecodeError::DuplicateKey(key.to_string()))
            },
            Header::Ref(v) => {
                match self.symbols.get(v) {
                    Some((Refable::Sym, symbol)) => Ok(Value::Symbol(symbol)),
                    Some((Refable::Key, key)) => Err(DecodeError::DuplicateKey(key.to_string())),
                    None => Err(DecodeError::UnknownRef(v))
                }
            },
        }
    }

    fn decode_slice(&mut self, len: usize) -> Result<&'a [u8], DecodeError> {
        if self.buf[self.pos..].len() < len {
            Err(DecodeError::Eof)
        } else {
            self.pos += len;
            Ok(&self.buf[self.pos - len .. self.pos])
        }
    }

}


#[cfg(test)]
mod test {
    use super::{Field, Value, Sign, Encoder, Decoder, DecodeError};

    #[test]
    fn simple_unnamed_fields() {
        let mut buf = Vec::new();
        assert_roundtrip(Field { name: None, value: Value::Null }, &mut buf);
        assert_roundtrip(Field { name: None, value: Value::Bool(true) }, &mut buf);
        assert_roundtrip(Field { name: None, value: Value::Bool(false) }, &mut buf);
        for i in (0..u64::MAX).step_by(3_203_431_780_337) {
            assert_roundtrip(Field { name: None, value: Value::Int(Sign::Pos, i) }, &mut buf);
            assert_roundtrip(Field { name: None, value: Value::Int(Sign::Neg, if i == 0 { 1 } else { i }) }, &mut buf);
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
        assert_roundtrip(Field { name: Some("null"), value: Value::Null }, &mut buf);
        assert_roundtrip(Field { name: Some("bool"), value: Value::Bool(true) }, &mut buf);
        assert_roundtrip(Field { name: Some("bool"), value: Value::Bool(false) }, &mut buf);
        for i in (0..u64::MAX).step_by(3_203_431_780_337) {
            assert_roundtrip(Field { name: Some("integer"), value: Value::Int(Sign::Pos, i) }, &mut buf);
            assert_roundtrip(Field { name: Some("integer"), value: Value::Int(Sign::Neg, if i == 0 { 1 } else { i }) }, &mut buf);
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
        for i in 0..1 << 10 {
            assert_roundtrip(Field { name: Some("array"), value: Value::Container(vec![
                    Field { name: None, value: Value::Int(Sign::Pos, 1) }; i as usize
            ])}, &mut buf);
        }
    }

    #[test]
    fn map() {
        let mut buf = Vec::new();
        assert_roundtrip(Field { name: Some("map"), value: Value::Container(vec![
                Field { name: Some("first"), value: Value::Int(Sign::Pos, 1) },
                Field { name: Some("second"), value: Value::Int(Sign::Pos, 2) },
                Field { name: Some("third"), value: Value::Int(Sign::Pos, 3) },
                Field { name: Some("fourth"), value: Value::Int(Sign::Pos, 4) },
        ])}, &mut buf);
    }

    #[test]
    fn symbols() {
        let mut buf = Vec::new();
        assert_roundtrip(Field { name: Some("array"), value: Value::Container(vec![
                Field { name: None, value: Value::Container(vec![ Field { name: Some("key"), value: Value::Symbol("VALUE") } ]) }; 3
            ])}, &mut buf);
    }

    #[test]
    fn errors() {
        let buf = [];
        assert!(matches!(Decoder::decode(&buf).unwrap_err().into_inner(), DecodeError::Eof));
        let buf = [4 << 5 | 2, 0xc3, 0x28];
        assert!(matches!(Decoder::decode(&buf).unwrap_err().into_inner(), DecodeError::Utf8(_)));
        let buf = [6 << 5 | 1, 0x21, 7 << 5 | 0];
        assert!(matches!(Decoder::decode(&buf).unwrap_err().into_inner(), DecodeError::DuplicateKey(key) if key == "!"));
        let buf = [7 << 5 | 0];
        assert!(matches!(Decoder::decode(&buf).unwrap_err().into_inner(), DecodeError::UnknownRef(0)));
    }

    fn assert_roundtrip(field: Field, buf: &mut Vec<u8>) {
        buf.clear();
        let _ = Encoder::encode(&field, buf);
        assert_eq!(field, Decoder::decode(buf).unwrap().0);
    }

}
