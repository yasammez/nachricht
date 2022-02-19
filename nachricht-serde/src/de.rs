use serde::{Deserialize};
use serde::de::{self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess, Visitor};
use nachricht::{DecodeError, Header, Refable, Sign};
use std::convert::TryInto;
use serde::de::value::StrDeserializer;

use crate::error::{DeserializationError, Error, Result};

/// Like a Header but with all symbol table references
/// resolved and inlined. More than a header less than a value.
enum Atom<'de> {
    Null,
    Bool(bool),
    F32(f32),
    F64(f64),
    Bin(usize),
    Int(i128),
    Str(&'de str),
    Sym(&'de str),
    Arr(usize),
    Rec(Vec<&'de str>),
    Map(usize),
}

impl<'de> Atom<'de> {
    fn name(&self) -> &'static str {
        match *self {
            Atom::Null => "Null",
            Atom::Bool(_) => "Bool",
            Atom::F32(_) => "F32",
            Atom::F64(_) => "F64",
            Atom::Bin(_) => "Bin",
            Atom::Int(_) => "Int",
            Atom::Str(_) => "Str",
            Atom::Sym(_) => "Sym",
            Atom::Arr(_) => "Arr",
            Atom::Rec(_) => "Rec",
            Atom::Map(_) => "Map",
        }
    }
}

pub struct Deserializer<'de> {
    input:  &'de [u8],
    pos: usize,
    symbols: Vec<Refable<'de>>,
}

impl<'de> Deserializer<'de> {
    pub fn from_bytes(input: &'de [u8]) -> Self {
        Deserializer { input, pos: 0, symbols: Vec::new() }
    }
}

pub fn from_bytes<'a, T: Deserialize<'a>>(s: &'a [u8]) -> std::result::Result<T, DeserializationError> {
    let mut deserializer = Deserializer::from_bytes(s);
    let t = T::deserialize(&mut deserializer).map_err(|e| e.at(deserializer.pos))?;
    if deserializer.input[deserializer.pos..].is_empty() {
        Ok(t)
    } else {
        Err(Error::Trailing.at(deserializer.pos))
    }
}

impl<'de> Deserializer<'de> {

    fn decode_atom(&mut self) -> Result<Atom<'de>> {
        let (header, c) = Header::decode(&self.input[self.pos..])?;
        self.pos += c;
        Ok(match header {
            Header::Null => Atom::Null,
            Header::True => Atom::Bool(true),
            Header::False => Atom::Bool(false),
            Header::F32 => Atom::F32(<f32>::from_be_bytes(self.decode_slice(4)?.try_into().unwrap())),
            Header::F64 => Atom::F64(<f64>::from_be_bytes(self.decode_slice(8)?.try_into().unwrap())),
            Header::Bin(v) => Atom::Bin(v),
            Header::Int(s, v) => Atom::Int(match s { Sign::Pos => 1, Sign::Neg => -1 } * v as i128),
            Header::Str(v) => Atom::Str(std::str::from_utf8(self.decode_slice(v)?)?),
            Header::Sym(v) => {
                let str = std::str::from_utf8(self.decode_slice(v)?)?;
                self.symbols.push(Refable::Sym(str));
                Atom::Sym(str)
            }
            Header::Arr(v) => Atom::Arr(v),
            Header::Rec(v) => {
                let mut lay = Vec::with_capacity(v);
                for _ in 0..v {
                    lay.push(self.decode_stringy()?);
                }
                self.symbols.push(Refable::Rec(lay.clone()));
                Atom::Rec(lay)
            }
            Header::Map(v) => Atom::Map(v),
            Header::Ref(v) => {
                match self.symbols.get(v) {
                    Some(Refable::Sym(s)) => Atom::Sym(s),
                    Some(Refable::Rec(s)) => Atom::Rec(s.clone()),
                    _ => { return Err(Error::Decode(DecodeError::InvalidRef(v))); },
                }
            }
        })
    }

    #[inline]
    fn decode_int(&mut self) -> Result<i128> {
        match self.decode_atom()? {
            Atom::Int(i) => Ok(i),
            o => Err(Error::UnexpectedHeader(&["Int"], o.name())),
        }
    }

    #[inline]
    fn decode_slice(&mut self, len: usize) -> Result<&'de [u8]> {
        if self.input[self.pos..].len() < len {
            Err(Error::Decode(DecodeError::Eof))
        } else {
            self.pos += len;
            Ok(&self.input[self.pos - len..self.pos])
        }
    }

    fn decode_stringy(&mut self) -> Result<&'de str> {
        match self.decode_atom()? {
            Atom::Str(v) | Atom::Sym(v) => Ok(v),
            o => Err(Error::UnexpectedHeader(&["Str", "Sym", "Ref"], o.name())),
        }
    }

}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.decode_atom()? {
            Atom::Null => visitor.visit_unit(),
            Atom::Bool(v) => visitor.visit_bool(v),
            Atom::F32(v) => visitor.visit_f32(v),
            Atom::F64(v) => visitor.visit_f64(v),
            Atom::Bin(v) => visitor.visit_borrowed_bytes(self.decode_slice(v)?),
            Atom::Int(v) => visitor.visit_i64(v.try_into()?),
            Atom::Str(v) => visitor.visit_borrowed_str(v),
            Atom::Sym(v) => visitor.visit_borrowed_str(v),
            Atom::Arr(v) => visitor.visit_seq(SeqDeserializer::new(self, v)),
            Atom::Map(v) => visitor.visit_map(MapDeserializer::new(self, v)),
            Atom::Rec(lay) => visitor.visit_map(StructDeserializer::new(self, lay)),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.decode_atom()? {
            Atom::Bool(v) => visitor.visit_bool(v),
            o => Err(Error::UnexpectedHeader(&["True", "False"], o.name())),
        }
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_i8(self.decode_int()?.try_into()?)
    }

    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_i16(self.decode_int()?.try_into()?)
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_i32(self.decode_int()?.try_into()?)
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_i64(self.decode_int()?.try_into()?)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u8(self.decode_int()?.try_into()?)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u16(self.decode_int()?.try_into()?)
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u32(self.decode_int()?.try_into()?)
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u64(self.decode_int()?.try_into()?)
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.decode_atom()? {
            Atom::F32(v) => visitor.visit_f32(v),
            o => Err(Error::UnexpectedHeader(&["F32"], o.name())),
        }
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.decode_atom()? {
            Atom::F64(v) => visitor.visit_f64(v),
            o => Err(Error::UnexpectedHeader(&["F64"], o.name())),
        }
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let v = self.decode_stringy()?;
        let mut chars = v.chars();
        let c = chars.next().ok_or(Error::Decode(DecodeError::Eof))?;
        match chars.next() {
            Some(_) => Err(Error::Trailing),
            None => visitor.visit_char(c),
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_borrowed_str(self.decode_stringy()?.as_ref())
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.decode_atom()? {
            Atom::Bin(v) => visitor.visit_borrowed_bytes(self.decode_slice(v)?),
            o => Err(Error::UnexpectedHeader(&["Bin"], o.name())),
        }
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.decode_atom()? {
            Atom::Bin(v) => visitor.visit_byte_buf(self.decode_slice(v)?.to_vec()),
            Atom::Arr(v) => {
                let mut bytes = Vec::with_capacity(v);
                for _ in 0..v {
                    bytes.push(self.decode_int()?.try_into()?);
                }
                visitor.visit_byte_buf(bytes)
            },
            o => Err(Error::UnexpectedHeader(&["Bin", "Arr"], o.name())),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let (header, c) = Header::decode(&self.input[self.pos..])?;
        match header {
            Header::Null => {
                self.pos += c;
                visitor.visit_none()
            },
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.decode_atom()? {
            Atom::Null => visitor.visit_unit(),
            o => Err(Error::UnexpectedHeader(&["Null"], o.name())),
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(mut self, visitor: V) -> Result<V::Value> {
        match self.decode_atom()? {
            Atom::Arr(v) => visitor.visit_seq(SeqDeserializer::new(&mut self, v)),
            o => Err(Error::UnexpectedHeader(&["Arr"], o.name())),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(self, _name: &'static str, _len: usize, visitor: V) -> Result<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(mut self, visitor: V) -> Result<V::Value> {
        match self.decode_atom()? {
            Atom::Map(v) => visitor.visit_map(MapDeserializer::new(&mut self, v)),
            o => Err(Error::UnexpectedHeader(&["Map"], o.name())),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(self, _name: &'static str, _fields: &'static [&'static str], visitor: V) -> Result<V::Value> {
        match self.decode_atom()? {
            Atom::Rec(lay) => visitor.visit_map(StructDeserializer::new(self, lay)),
            o => Err(Error::UnexpectedHeader(&["Rec", "Ref"], o.name())),
        }
    }

    fn deserialize_enum<V: Visitor<'de>>(self, _name: &'static str, _variants: &'static [&'static str],  visitor: V) -> Result<V::Value> {
        match self.decode_atom()? {
            Atom::Rec(lay) if lay.len() == 1 => {
                let variant = lay[0];
                visitor.visit_enum(EnumDeserializer::new(self, variant))
            },
            Atom::Sym(s) => visitor.visit_enum(s.into_deserializer()),
            Atom::Str(s) => visitor.visit_enum(s.into_deserializer()),
            o => Err(Error::UnexpectedHeader(&["Rec", "Ref", "Str", "Sym"], o.name())),
        }
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_borrowed_str(self.decode_stringy()?.as_ref())
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_any(visitor)
    }

}

struct MapDeserializer<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    remaining: usize,
}

impl<'a, 'de> MapDeserializer<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, remaining: usize) -> Self {
        Self { de, remaining }
    }
}

impl<'de, 'a> MapAccess<'de> for MapDeserializer<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        if self.remaining == 0 {
            Ok(None)
        } else {
            self.remaining -= 1;
            seed.deserialize(&mut *self.de).map(Some)
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        seed.deserialize(&mut *self.de)
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        Some(self.remaining)
    }
}

struct StructDeserializer<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    layout: Vec<&'de str>,
    pos: usize,
}

impl<'a, 'de> StructDeserializer<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, layout: Vec<&'de str>) -> Self {
        Self { de, layout, pos: 0 }
    }
}

impl<'de, 'a> MapAccess<'de> for StructDeserializer<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        if self.pos == self.layout.len() {
            Ok(None)
        } else {
            self.pos += 1;
            seed.deserialize(self.layout[self.pos - 1].into_deserializer()).map(Some)
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        seed.deserialize(&mut *self.de)
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        Some(self.layout.len() - self.pos)
    }
}

struct EnumDeserializer<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    variant: &'de str,
}

impl<'a, 'de> EnumDeserializer<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, variant: &'de str) -> Self {
        Self { de, variant }
    }
}

impl<'de, 'a> EnumAccess<'de> for EnumDeserializer<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)> {
        let deserializer: StrDeserializer<'de, Error> = self.variant.into_deserializer();
        let variant = seed.deserialize(deserializer)?;
        Ok((variant, self))
    }
}

impl<'de, 'a> VariantAccess<'de> for EnumDeserializer<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        match self.de.decode_atom()? {
            Atom::Null => Ok(()),
            o => Err(Error::UnexpectedHeader(&["Null"], o.name())),
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        seed.deserialize(self.de)
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value> {
        de::Deserializer::deserialize_seq(self.de, visitor)
    }

    fn struct_variant<V: Visitor<'de>>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value> {
        de::Deserializer::deserialize_struct(self.de, "", fields, visitor)
    }

}

struct SeqDeserializer<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    remaining: usize,
}

impl<'a, 'de> SeqDeserializer<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, remaining: usize) -> Self {
        Self { de, remaining }
    }
}

impl<'de, 'a> SeqAccess<'de> for SeqDeserializer<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        if self.remaining == 0 {
            Ok(None)
        } else {
            self.remaining -= 1;
            seed.deserialize(&mut *self.de).map(Some)
        }
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        Some(self.remaining)
    }

}