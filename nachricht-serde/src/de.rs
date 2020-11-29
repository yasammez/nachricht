use serde::Deserialize;
use serde::de::{self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess, Visitor};
use nachricht::*;
use std::convert::TryInto;

use crate::error::{Error, Result};

pub struct Deserializer<'de> {
    input:  &'de [u8],
}

impl<'de> Deserializer<'de> {
    pub fn from_bytes(input: &'de [u8]) -> Self {
        Deserializer { input }
    }
}

pub fn from_bytes<'a, T: Deserialize<'a>>(s: &'a [u8]) -> Result<T> {
    let mut deserializer = Deserializer::from_bytes(s);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.input.is_empty() {
        Ok(t)
    } else {
        Err(Error::Trailing)
    }
}

impl<'de> Deserializer<'de> {
    fn decode_int(&mut self) -> Result<i128> {
        let (field, tail) = Field::decode(self.input)?;
        self.input = tail;
        match &field.value {
            Value::Int(s,val) => Ok(match s { Sign::Pos => *val as i128, Sign::Neg => -(*val as i128) }),
            _ => Err(Error::Unexpected),
        }
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let (field, tail) = Field::decode(self.input)?;
        // Don't advance the cursor
        // TODO: This is (very) slow for containers!
        match field.value {
            Value::Unit => self.deserialize_unit(visitor),
            Value::Bool(_) => self.deserialize_bool(visitor),
            Value::F32(_) => self.deserialize_f32(visitor),
            Value::F64(_) => self.deserialize_f64(visitor),
            Value::Bytes(_) => self.deserialize_bytes(visitor),
            Value::Int(_,_) => self.deserialize_i64(visitor),
            Value::Str(_) => self.deserialize_str(visitor),
            Value::Container(_) => self.deserialize_seq(visitor), // TODO: wie unterscheide ich zwischen seq, map, struct und enum?
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        // TODO: refactor this, it is repeated a lot
        let (field, tail) = Field::decode(self.input)?;
        self.input = tail;
        match &field.value {
            Value::Bool(value) => visitor.visit_bool(*value),
            _ => Err(Error::Unexpected),
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
        let (field, tail) = Field::decode(self.input)?;
        self.input = tail;
        match &field.value {
            Value::F32(value) => visitor.visit_f32(*value),
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let (field, tail) = Field::decode(self.input)?;
        self.input = tail;
        match &field.value {
            Value::F64(value) => visitor.visit_f64(*value),
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let (field, tail) = Field::decode(self.input)?;
        self.input = tail;
        match &field.value {
            Value::Str(value) if value.len() == 1 => visitor.visit_char(value.chars().next().unwrap()),
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let (field, tail) = Field::decode(self.input)?;
        self.input = tail;
        match &field.value {
            Value::Str(value) => visitor.visit_borrowed_str(value),
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let (field, tail) = Field::decode(self.input)?;
        self.input = tail;
        match &field.value {
            Value::Bytes(value) => visitor.visit_bytes(value),
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let (field, tail) = Field::decode(self.input)?;
        self.input = tail;
        match &field.value {
            Value::Bytes(value) => visitor.visit_byte_buf(value.to_vec()),
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let (field, tail) = Field::decode(self.input)?;
        // TODO: Don't advance the buffer ... probably?
        match &field.value {
            Value::Unit => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let (field, tail) = Field::decode(self.input)?;
        self.input = tail;
        match &field.value {
            Value::Unit => visitor.visit_unit(),
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(self, name: &'static str, visitor: V) -> Result<V::Value> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, name: &'static str, visitor: V) -> Result<V::Value> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(mut self, visitor: V) -> Result<V::Value> {
        let (Header(code, value), tail) = Header::decode(self.input)?;
        self.input = tail;
        match code {
            Code::Container => visitor.visit_seq(SeqDeserializer::new(&mut self, value)),
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(self, name: &'static str, len: usize, visitor: V) -> Result<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(mut self, visitor: V) -> Result<V::Value> {
        let (Header(code, value), tail) = Header::decode(self.input)?;
        self.input = tail;
        match code {
            Code::Container => visitor.visit_map(MapDeserializer::new(&mut self, value)),
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(mut self, name: &'static str, fields: &'static [&'static str], visitor: V) -> Result<V::Value> {
        let (Header(code, value), tail) = Header::decode(self.input)?;
        self.input = tail;
        match code {
            Code::Container => visitor.visit_map(StructDeserializer::new(&mut self, value)),
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_enum<V: Visitor<'de>>(self, name: &'static str, variants: &'static [&'static str],  visitor: V) -> Result<V::Value> {
        let (Header(code, value), tail) = Header::decode(self.input)?;
        self.input = tail;
        match code {
            Code::Str => {
                let name = std::str::from_utf8(&self.input[..value as usize]).map_err(|e| <DecodeError>::from(e))?; // TODO: panic! im slice access
                visitor.visit_enum(name.into_deserializer())
            },
            Code::Container => visitor.visit_enum(EnumDeserializer::new(&mut *self)),
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let (Header(code, value), tail) = Header::decode(self.input)?;
        self.input = tail;
        match code {
            Code::Key => {
                let string = std::str::from_utf8(&self.input[..value as usize]).map_err(|e| <DecodeError>::from(e))?; // TODO: panic! im slice access
                visitor.visit_borrowed_str(string)
            },
            _ => Err(Error::Unexpected)
        }
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let (field, tail) = Field::decode(self.input)?; // TODO: Wenn Header.1 für Container die Länge in Bytes enthielte müsste man den Inhalt nicht dekodieren
        self.input = tail;
        visitor.visit_bool(true)
    }

}

struct MapDeserializer<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    remaining: u64,
}

impl<'a, 'de> MapDeserializer<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, remaining: u64) -> Self {
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

}

struct StructDeserializer<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    remaining: u64,
}

impl<'a, 'de> StructDeserializer<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, remaining: u64) -> Self {
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
            let (Header(code, value), tail) = Header::decode(self.de.input)?;
            match code {
                Code::Key => seed.deserialize(&mut *self.de).map(Some), // TODO: wie gebe ich den Inhalt des Keys weiter?
                _ => Err(Error::Unexpected)
            }
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

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    {
        let value = seed.deserialize(&mut *self.de)?;
        Ok((value, self))
    }
}

impl<'de, 'a> VariantAccess<'de> for EnumDeserializer<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Err(Error::Unexpected)
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        seed.deserialize(self.de)
    }

    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value> {
        de::Deserializer::deserialize_seq(self.de, visitor)
    }

    fn struct_variant<V: Visitor<'de>>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value> {
        de::Deserializer::deserialize_map(self.de, visitor)
    }

}

struct SeqDeserializer<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    remaining: u64,
}

impl<'a, 'de> SeqDeserializer<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, remaining: u64) -> Self {
        Self { de, remaining }
    }
}

impl<'de, 'a> SeqAccess<'de> for SeqDeserializer<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        if self.remaining == 0 {
            Ok(None)
        } else {
            seed.deserialize(&mut *self.de).map(Some)
        }
    }

}
