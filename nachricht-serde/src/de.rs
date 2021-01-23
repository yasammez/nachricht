use serde::Deserialize;
use serde::de::{self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess, Visitor};
use nachricht::{DecodeError, Header, Refable};
use std::convert::TryInto;

use crate::error::{DeserializationError, Error, Result};

pub struct Deserializer<'de> {
    input:  &'de [u8],
    pos: usize,
    symbols: Vec<(Refable, &'de str)>,
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

    #[inline]
    fn decode_header(&mut self) -> Result<Header> {
        let (header, c) = Header::decode(&self.input[self.pos..])?;
        self.pos += c;
        Ok(header)
    }

    #[inline]
    fn decode_int(&mut self) -> Result<i128> {
        let header = self.decode_header()?;
        match header {
            Header::Pos(v) => Ok(v as i128),
            Header::Neg(v) => Ok(-(v as i128)),
            o => Err(Error::UnexpectedHeader(&["Pos", "Neg"], o.name())),
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

    fn decode_stringy(&mut self, header: Header) -> Result<(&'de str, Refable)> {
        match header {
            Header::Str(v) => Ok((std::str::from_utf8(self.decode_slice(v)?)?, Refable::Sym)),
            Header::Sym(v) => {
                let sym = std::str::from_utf8(self.decode_slice(v)?)?;
                self.symbols.push((Refable::Sym, sym));
                Ok((sym, Refable::Sym))
            },
            Header::Key(v) => {
                let key = std::str::from_utf8(self.decode_slice(v)?)?;
                self.symbols.push((Refable::Key, key));
                Ok((key, Refable::Key))
            },
            Header::Ref(v) => {
                match self.symbols.get(v) {
                    Some((refable, v)) => Ok((v, *refable)),
                    None => Err(Error::Decode(DecodeError::UnknownRef(v))),
                }
            },
            o => Err(Error::UnexpectedHeader(&["Str", "Sym", "Key", "Ref"], o.name())),
        }
    }

}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let header = self.decode_header()?;
        match header {
            Header::Null => visitor.visit_unit(),
            Header::True => visitor.visit_bool(true),
            Header::False => visitor.visit_bool(false),
            Header::F32 => visitor.visit_f32(<f32>::from_be_bytes(self.decode_slice(4)?.try_into().unwrap())),
            Header::F64 => visitor.visit_f64(<f64>::from_be_bytes(self.decode_slice(8)?.try_into().unwrap())),
            Header::Bin(v) => visitor.visit_borrowed_bytes(self.decode_slice(v)?),
            Header::Pos(v) => visitor.visit_u64(v),
            Header::Neg(v) => visitor.visit_i64(-(v as i128).try_into()?),
            Header::Str(v) => visitor.visit_borrowed_str(std::str::from_utf8(self.decode_slice(v)?)?),
            Header::Sym(v) => {
                let sym = std::str::from_utf8(self.decode_slice(v)?)?;
                self.symbols.push((Refable::Sym, sym));
                visitor.visit_borrowed_str(sym)
            },
            Header::Bag(v) => visitor.visit_seq(SeqDeserializer::new(self, v)),
            Header::Key(v) => {
                let key = std::str::from_utf8(self.decode_slice(v)?)?;
                self.symbols.push((Refable::Key, key));
                visitor.visit_borrowed_str(key)
            },
            Header::Ref(v) => {
                match self.symbols.get(v) {
                    Some((_, v)) => visitor.visit_borrowed_str(v),
                    None => Err(Error::Decode(DecodeError::UnknownRef(v))),
                }
            },
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.decode_header()? {
            Header::True => visitor.visit_bool(true),
            Header::False => visitor.visit_bool(false),
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
        match self.decode_header()? {
            Header::F32 => visitor.visit_f32(<f32>::from_be_bytes(self.decode_slice(4)?.try_into().unwrap())),
            o => Err(Error::UnexpectedHeader(&["F32"], o.name())),
        }
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.decode_header()? {
            Header::F64 => visitor.visit_f64(<f64>::from_be_bytes(self.decode_slice(8)?.try_into().unwrap())),
            o => Err(Error::UnexpectedHeader(&["F64"], o.name())),
        }
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let header = self.decode_header()?;
        match self.decode_stringy(header)? {
            (v, Refable::Sym) => {
                let mut chars = v.chars();
                let c = chars.next().ok_or(Error::Decode(DecodeError::Eof))?;
                match chars.next() {
                    Some(_) => Err(Error::Trailing),
                    None => visitor.visit_char(c),
                }
            },
            (_, o) => Err(Error::UnexpectedRefable(Refable::Sym.name(), o.name())),
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let header = self.decode_header()?;
        match self.decode_stringy(header)? {
            (v, Refable::Sym) => visitor.visit_borrowed_str(v),
            (_, o) => Err(Error::UnexpectedRefable(Refable::Sym.name(), o.name())),
        }
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let header = self.decode_header()?;
        match header {
            Header::Bin(v) => visitor.visit_borrowed_bytes(self.decode_slice(v)?),
            o => Err(Error::UnexpectedHeader(&["Bin"], o.name())),
        }
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.decode_header()? {
            Header::Bin(v) => visitor.visit_byte_buf(self.decode_slice(v)?.to_vec()),
            Header::Bag(v) => {
                let mut bytes = Vec::with_capacity(v);
                for _ in 0..v {
                    bytes.push(self.decode_int()?.try_into()?);
                }
                visitor.visit_byte_buf(bytes)
            },
            o => Err(Error::UnexpectedHeader(&["Bin", "Bag"], o.name())),
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
        match self.decode_header()? {
            Header::Null => visitor.visit_unit(),
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
        match self.decode_header()? {
            Header::Bag(v) => visitor.visit_seq(SeqDeserializer::new(&mut self, v)),
            o => Err(Error::UnexpectedHeader(&["Bag"], o.name())),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(self, _name: &'static str, _len: usize, visitor: V) -> Result<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(mut self, visitor: V) -> Result<V::Value> {
        match self.decode_header()? {
            Header::Bag(v) => visitor.visit_map(MapDeserializer::new(&mut self, v)),
            o => Err(Error::UnexpectedHeader(&["Bag"], o.name())),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(mut self, _name: &'static str, _fields: &'static [&'static str], visitor: V) -> Result<V::Value> {
        match self.decode_header()? {
            Header::Bag(v) => visitor.visit_map(StructDeserializer::new(&mut self, v)),
            o => Err(Error::UnexpectedHeader(&["Bag"], o.name())),
        }
    }

    fn deserialize_enum<V: Visitor<'de>>(self, _name: &'static str, _variants: &'static [&'static str],  visitor: V) -> Result<V::Value> {
        match self.decode_header()? {
            Header::Bag(1) => visitor.visit_enum(EnumDeserializer::new(&mut *self)),
            h => match self.decode_stringy(h)? {
                (v, Refable::Sym) => visitor.visit_enum(v.into_deserializer()),
                (_, o) => Err(Error::UnexpectedRefable(Refable::Sym.name(), o.name())),
            },
        }
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let header = self.decode_header()?;
        match self.decode_stringy(header)? {
            (v, Refable::Key) => visitor.visit_borrowed_str(v),
            (_, o) => Err(Error::UnexpectedRefable(Refable::Key.name(), o.name())),
        }
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
        if self.remaining == 0 {
            Err(Error::Trailing)
        } else {
            self.remaining -= 1;
            seed.deserialize(&mut *self.de)
        }
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        Some(self.remaining >> 1)
    }
}

struct StructDeserializer<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    remaining: usize,
}

impl<'a, 'de> StructDeserializer<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, remaining: usize) -> Self {
        Self { de, remaining }
    }
}

impl<'de, 'a> MapAccess<'de> for StructDeserializer<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        if self.remaining == 0 {
            Ok(None)
        } else {
            self.remaining -= 1;
            let header = self.de.decode_header()?;
            match self.de.decode_stringy(header)? {
                (v, Refable::Key) => seed.deserialize(v.into_deserializer()).map(Some),
                (_, o) => Err(Error::UnexpectedRefable(Refable::Key.name(), o.name())),
            }
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

struct EnumDeserializer<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> EnumDeserializer<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Self { de }
    }
}

impl<'de, 'a> EnumAccess<'de> for EnumDeserializer<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)> {
        let variant = seed.deserialize(&mut *self.de)?;
        Ok((variant, self))
    }
}

impl<'de, 'a> VariantAccess<'de> for EnumDeserializer<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        match self.de.decode_header()? {
            Header::Null => Ok(()),
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
