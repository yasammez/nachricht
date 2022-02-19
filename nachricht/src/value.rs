//! The atom of a `nachricht` is the `Value`.
//! Values are encoded on wire as headers and, if necessary, additional bytes which directly follow the header. Record layouts and
//! values with datatype `Value::Symbol` are defined within a symbol table an can be referenced later within the wire format,
//! so you pay their full bandwidth costs only once. This encoding is transparent, there is no need
//! to manually define a symbol table within the model.

use crate::header::{Header, Sign};
use crate::error::{DecodeError, DecoderError, EncodeError};
use std::mem::size_of;
use std::io::Write;
use std::convert::TryInto;
use std::str::from_utf8;
use std::iter::repeat;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};

/// The possible values according to the `nachricht` data model.
#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    Null,
    Bool(bool),
    F32(f32),
    F64(f64),
    Bytes(Cow<'a, [u8]>),
    Int(Sign, u64),
    Str(Cow<'a, str>),
    Symbol(Cow<'a, str>),
    Record(BTreeMap<Cow<'a, str>, Value<'a>>),
    Map(Vec<(Value<'a>, Value<'a>)>),
    Array(Vec<Value<'a>>),
}

impl<'a> Value<'a> {

    const PROTECTED_CHARS: &'static str = "\n\\$ ,:\"'()[]{}#";

    fn b64(input: &[u8]) -> String {
        const CHAR_SET: &'static [char] = &['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N',
            'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g',
            'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
            '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '+', '/'
        ];
        let mut array = [0; 4];
        input.chunks(3).flat_map(|chunk| {
            let len = chunk.len();
            array[1..1 + len].copy_from_slice(chunk);
            for i in 0..(3 - len) {
                array[3 - i] = 0;
            }
            let x = u32::from_be_bytes(array);
            (0..=len).map(move |o| CHAR_SET[(x >> (18 - 6*o) & 0x3f) as usize]).chain(repeat('=').take(3-len))
        }).collect()
    }

    fn typename(&self) -> &'static str {
        match *self {
            Self::Null      => "null",
            Self::Bool(_)   => "bool",
            Self::F32(_)    => "f32",
            Self::F64(_)    => "f64",
            Self::Bytes(_)  => "bytes",
            Self::Int(_, _) => "integer",
            Self::Str(_)    => "string",
            Self::Symbol(_) => "symbol",
            Self::Record(_) => "record",
            Self::Map(_)    => "map",
            Self::Array(_)  => "array",
        }
    }

}



impl<'a> std::fmt::Display for Value<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null         => f.write_str("null"),
            Value::Bool(true)   => f.write_str("true"),
            Value::Bool(false)  => f.write_str("false"),
            Value::F32(v)       => write!(f, "${}", v),
            Value::F64(v)       => write!(f, "$${}", v),
            Value::Bytes(v)     => write!(f, "'{}'", Self::b64(v).as_str()),
            Value::Int(s, v)    => write!(f, "{}{}", match s { Sign::Pos => "", Sign::Neg => "-" }, v),
            Value::Str(v)       => write!(f, "\"{}\"", v.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n")),
            Value::Symbol(v) if v.chars().any(|c| Self::PROTECTED_CHARS.contains(c))
                                => write!(f, "#\"{}\"", v.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n")),
            Value::Symbol(v)    => write!(f, "#{}", v),
            Value::Record(v)    => write!(f, "(\n{}\n)", v.iter()
                .flat_map(|(k, f)| format!("{}: {},", if k.chars().any(|c| Self::PROTECTED_CHARS.contains(c)) {
                    format!("\"{}\"", k.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n"))
                } else {
                    format!("{}", k )
                }, f).lines().map(|line| format!("  {}", line)).collect::<Vec<String>>())
                .collect::<Vec<String>>().join("\n")),
            Value::Map(v)       => write!(f, "{{\n{}\n}}", v.iter()
                .flat_map(|(k, f)| format!("{}: {},", k, f).lines().map(|line| format!("  {}", line)).collect::<Vec<String>>())
                .collect::<Vec<String>>().join("\n")),
            Value::Array(v)    => write!(f, "[\n{}\n]", v.iter()
                .flat_map(|f| format!("{},", f).lines().map(|line| format!("  {}", line)).collect::<Vec<String>>())
                .collect::<Vec<String>>().join("\n")),
        }
    }
}

#[derive(PartialEq, Clone)]
#[repr(u8)]
pub enum Refable<'a> {
    Sym(&'a str),
    Rec(Vec<&'a str>),
}

impl<'a> Refable<'a> {
    pub fn name(&self) -> &'static str {
        match *self {
            Refable::Sym(_) => "Sym",
            Refable::Rec(_) => "Rec",
        }
    }
}

/// Used to encode `nachricht` fields. This uses a symbol table to allow referencing symbols and
/// record layouts which get repeated.
pub struct Encoder<'w, W: Write> {
    writer: &'w mut W,
    /// Next free value to insert into the table
    next_free: usize,
    /// Map symbol -> entry in the table
    symbols: HashMap<Cow<'w, str>, usize>,
    /// Map record -> entry in the table
    records: HashMap<Vec<Cow<'w, str>>, usize>,
}

impl<'w, W: Write> Encoder<'w, W> {

    /// Encode a field to the given writer. The resulting `usize` is the amount of bytes that got written.
    pub fn encode(field: &'w Value, writer: &'w mut W) -> Result<usize, EncodeError> {
        Self { writer, symbols: HashMap::new(), records: HashMap::new(), next_free: 0 }.encode_inner(field)
    }

    fn encode_inner(&mut self, field: &'w Value) -> Result<usize, EncodeError> {
        let mut c = 0;
        match &field {
            Value::Null        => Header::Null.encode(self.writer),
            Value::Bool(true)  => Header::True.encode(self.writer),
            Value::Bool(false) => Header::False.encode(self.writer),
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
            Value::Int(s, v) => Header::Int(*s, *v).encode(self.writer),
            Value::Str(v) => {
                c += Header::Str(v.len()).encode(self.writer)?;
                self.writer.write_all(v.as_bytes())?;
                Ok(c + v.len())
            },
            Value::Symbol(v) => self.encode_symbol(v),
            Value::Array(inner) => {
                c += Header::Arr(inner.len()).encode(self.writer)?;
                for field in inner.iter() {
                    c += self.encode_inner(field)?;
                }
                Ok(c)
            },
            Value::Record(inner) => self.encode_record(inner),
            Value::Map(inner) => {
                c += Header::Map(inner.len()).encode(self.writer)?;
                for (key, val) in inner.iter() {
                    c += self.encode_inner(key)?;
                    c += self.encode_inner(val)?;
                }
                Ok(c)
            },
        }
    }

    fn encode_record(&mut self, inner: &'w BTreeMap<Cow<'w, str>, Value<'w>>) -> Result<usize, EncodeError> {
        let mut c = match self.records.get(&inner.keys().map(|i| i.clone()).collect::<Vec<_>>()) {
            Some(i) => Header::Ref(*i).encode(self.writer)?,
            None    => {
                let mut x = Header::Rec(inner.len()).encode(self.writer)?;
                for sym in inner.keys() {
                    x += self.encode_symbol(sym)?;
                }
                let index = self.next();
                self.records.insert(inner.keys().map(|i| i.clone()).collect(), index);
                x
            }
        };
        for val in inner.values() {
            c += self.encode_inner(val)?;
        }
        Ok(c)
    }

    fn encode_symbol(&mut self, symbol: &'w str) -> Result<usize, EncodeError> {
        match self.symbols.get(symbol) {
            Some(i) => Header::Ref(*i).encode(self.writer),
            None    => {
                let index = self.next();
                self.symbols.insert(symbol.into(), index);
                let c = Header::Sym(symbol.len()).encode(self.writer)?;
                self.writer.write_all(symbol.as_bytes())?;
                Ok(c + symbol.len())
            }
        }
    }

    fn next(&mut self) -> usize {
        self.next_free += 1;
        self.next_free - 1
    }

}
/// Used to decode `nachricht` fields. This uses a symbol table to allow the decoding of encountered references.
pub struct Decoder<'a> {
    symbols: Vec<Refable<'a>>,
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Decoder<'a> {

    /// Decode a single value from the given buffer. All strings, keys, symbols and byte data will be borrowed from the
    /// buffer instead of copied. This means that the decoded field may only live as long as the buffer does. However,
    /// some allocations still occur: containers need their own heap space.
    pub fn decode<B: ?Sized + AsRef<[u8]>>(buf: &'a B) -> Result<(Value<'a>, usize), DecoderError> {
        let mut decoder = Self { buf: buf.as_ref(), symbols: Vec::new(), pos: 0 };
        let value = decoder.decode_value().map_err(|e| e.at(decoder.pos))?;
        Ok((value, decoder.pos))
    }

    fn decode_value(&mut self) -> Result<Value<'a>, DecodeError> {
        let header = self.decode_header()?;
        match header {
            Header::Null      => Ok(Value::Null),
            Header::True      => Ok(Value::Bool(true)),
            Header::False     => Ok(Value::Bool(false)),
            Header::F32       => Ok(Value::F32(<f32>::from_be_bytes(self.decode_slice(4)?.try_into().unwrap()))),
            Header::F64       => Ok(Value::F64(<f64>::from_be_bytes(self.decode_slice(8)?.try_into().unwrap()))),
            Header::Bin(v)    => Ok(Value::Bytes(Cow::Borrowed(self.decode_slice(v)?))),
            Header::Int(s, v) => Ok(Value::Int(s, v)),
            Header::Arr(v) => {
                let mut elements = Vec::with_capacity(0);
                elements.try_reserve(v)?;
                for _ in 0..v {
                    elements.push(self.decode_value()?);
                }
                Ok(Value::Array(elements))
            },
            Header::Map(v) => {
                let mut elements = Vec::with_capacity(0);
                elements.try_reserve(v)?;
                for _ in 0..v {
                    let key = self.decode_value()?;
                    let val = self.decode_value()?;
                    elements.push((key, val));
                }
                Ok(Value::Map(elements))
            }
            Header::Str(v) => Ok(Value::Str(Cow::Borrowed(from_utf8(&self.decode_slice(v)?)?))),
            Header::Sym(v) => {
                let sym = from_utf8(&self.decode_slice(v)?)?;
                self.symbols.push(Refable::Sym(sym));
                Ok(Value::Symbol(Cow::Borrowed(sym)))
            },
            Header::Rec(v) => {
                let mut fields = BTreeMap::new();
                let mut keys = Vec::with_capacity(0);
                keys.try_reserve(v)?;
                for _ in 0..v {
                    match self.decode_value()? {
                        Value::Symbol(Cow::Borrowed(sym)) => { keys.push(sym); },
                        x => { return Err(DecodeError::IllegalKey(x.typename())); }
                    }
                }
                self.symbols.push(Refable::Rec(keys.clone()));
                for key in keys {
                    let val = self.decode_value()?;
                    fields.insert(Cow::Borrowed(key), val);
                }
                Ok(Value::Record(fields))
            },
            Header::Ref(v) => {
                match self.symbols.get(v) {
                    Some(Refable::Sym(s)) => Ok(Value::Symbol(Cow::Borrowed(s))),
                    Some(Refable::Rec(ref s)) => {
                        let mut fields = BTreeMap::<Cow<'a, str>, Value<'a>>::new();
                        for key in s.clone() {
                            fields.insert(Cow::Borrowed(key), self.decode_value()?);
                        }
                        Ok(Value::Record(fields))
                    }
                    None => Err(DecodeError::InvalidRef(v))
                }
            },
        }
    }

    fn decode_header(&mut self) -> Result<Header, DecodeError> {
        let (header, c) = Header::decode(&self.buf[self.pos..])?;
        self.pos += c;
        Ok(header)
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
    use super::{Value, Sign, Encoder, Decoder, DecodeError};
    use std::borrow::Cow;
    use std::collections::BTreeMap;

    #[test]
    fn simple_values() {
        let mut buf = Vec::new();
        assert_roundtrip(Value::Null, &mut buf);
        assert_roundtrip(Value::Bool(true), &mut buf);
        assert_roundtrip(Value::Bool(false), &mut buf);
        for i in (0..u64::MAX).step_by(3_203_431_780_337) {
            assert_roundtrip(Value::Int(Sign::Pos, i), &mut buf);
            assert_roundtrip(Value::Int(Sign::Neg, if i == 0 { 1 } else { i }), &mut buf);
        }
    }

    #[test]
    fn floats() {
        let mut buf = Vec::new();
        assert_roundtrip(Value::F64(f64::MAX), &mut buf);
        assert_roundtrip(Value::F64(f64::MIN), &mut buf);
        assert_roundtrip(Value::F64(std::f64::consts::PI), &mut buf);
        assert_roundtrip(Value::F32(f32::MAX), &mut buf);
        assert_roundtrip(Value::F32(f32::MIN), &mut buf);
        assert_roundtrip(Value::F32(std::f32::consts::PI), &mut buf);
    }

    #[test]
    fn strings() {
        let mut buf = Vec::new();
        assert_roundtrip(Value::Str(Cow::Borrowed("Üben von Xylophon und Querflöte ist ja zweckmäßig.")), &mut buf);
    }

    #[test]
    fn symbols() {
        let mut buf = Vec::new();
        assert_roundtrip(Value::Array(vec![
                Value::Symbol(Cow::Borrowed("PrionailurusViverrinus")),
                Value::Symbol(Cow::Borrowed("PrionailurusViverrinus")),
                Value::Symbol(Cow::Borrowed("PrionailurusViverrinus")),
                Value::Symbol(Cow::Borrowed("PrionailurusViverrinus")),
        ]), &mut buf);
    }

    #[test]
    fn bytes() {
        let mut buf = Vec::new();
        assert_roundtrip(Value::Bytes(Cow::Borrowed(&[1, 2, 3, 4, 255])), &mut buf);
    }

    #[test]
    fn array_mixed() {
        let mut buf = Vec::new();
        assert_roundtrip(Value::Array(vec![
                Value::Int(Sign::Pos, 1),
                Value::Str(Cow::Borrowed("Jessica")),
                Value::Symbol(Cow::Borrowed("FelisCatus")),
                Value::F32(std::f32::consts::PI),
        ]), &mut buf);
    }

    #[test]
    fn array_long() {
        let mut buf = Vec::new();
        for i in 0..1 << 10 {
            assert_roundtrip(Value::Array(vec![ Value::Int(Sign::Pos, 1); i as usize ]), &mut buf);
        }
    }

    #[test]
    fn map() {
        let mut buf = Vec::new();
        assert_roundtrip(Value::Map(vec![
                (Value::Str(Cow::Borrowed("first")),  Value::Int(Sign::Pos, 1)),
                (Value::Str(Cow::Borrowed("second")), Value::Int(Sign::Pos, 2)),
                (Value::Str(Cow::Borrowed("third")),  Value::Int(Sign::Pos, 3)),
                (Value::Str(Cow::Borrowed("fourth")), Value::Int(Sign::Pos, 4)),
        ]), &mut buf);
    }

    #[test]
    fn record() {
        let mut buf = Vec::new();
        assert_roundtrip(Value::Array(vec![
                Value::Record(BTreeMap::from([
                        (Cow::Borrowed("name"), Value::Str(Cow::Borrowed("Jessica"))),
                        (Cow::Borrowed("species"), Value::Symbol(Cow::Borrowed("PrionailurusViverrinus"))),
                ])),
                Value::Record(BTreeMap::from([
                        (Cow::Borrowed("name"), Value::Str(Cow::Borrowed("Wantan"))),
                        (Cow::Borrowed("species"), Value::Symbol(Cow::Borrowed("LynxLynx"))),
                ])),
        ]), &mut buf);
    }

    #[test]
    fn errors() {
        let buf = [];
        assert!(matches!(Decoder::decode(&buf).unwrap_err().into_inner(), DecodeError::Eof));
        let buf = [2 << 5 | 2, 0xc3, 0x28];
        assert!(matches!(Decoder::decode(&buf).unwrap_err().into_inner(), DecodeError::Utf8(_)));
        let buf = [7 << 5 | 0];
        assert!(matches!(Decoder::decode(&buf).unwrap_err().into_inner(), DecodeError::InvalidRef(0)));
        let buf = [5 << 5 | 1, 5 << 5];
        assert!(matches!(dbg!(Decoder::decode(&buf)).unwrap_err().into_inner(), DecodeError::IllegalKey("record")));
    }

    #[test]
    fn too_big_allocations() {
        let mut buf = [0u8; 9];
        buf[0] = 0x7f;
        for i in (1..u64::MAX).step_by(3_203_431_780_337) {
                let i = i.to_be_bytes();
                buf[1..].copy_from_slice(&i[..]);
                assert!(Decoder::decode(&buf).is_err()); // should never panic
        }
    }

    #[test]
    fn display_record_key() {
        let value = Value::Record(BTreeMap::from([(Cow::Borrowed("true or false"), Value::Bool(false))]));
        assert_eq!("(\n  \"true or false\": false,\n)", format!("{}", &value));
    }

    fn assert_roundtrip(val: Value, buf: &mut Vec<u8>) {
        buf.clear();
        let _ = Encoder::encode(&val, buf);
        assert_eq!(val, Decoder::decode(buf).unwrap().0);
    }

}
