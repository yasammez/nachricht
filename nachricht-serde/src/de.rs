use serde::Deserialize;
use serde::de::{self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess, Visitor};
use nachricht::{DecodeError, Code, Fixed, Header};
use nachricht::{Field, Value}; // TODO: loswerden
use std::convert::{TryInto, TryFrom};

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

    #[inline]
    fn decode_header(&mut self) -> Result<(Code, u64)> {
        let (Header(code, value), tail) = Header::decode(self.input)?;
        self.input = tail;
        Ok((code, value))
    }

    #[inline]
    fn decode_fixed(&mut self) -> Result<Fixed> {
        let (code, value) = self.decode_header()?;
        match code {
            Code::Fixed => Ok(Fixed::from_bits(value)?),
            _ => Err(Error::Unexpected)
        }
    }

    #[inline]
    fn decode_int(&mut self) -> Result<i128> {
        let (code, value) = self.decode_header()?;
        match code {
            Code::Intp => Ok(value as i128),
            Code::Intn => Ok(-(value as i128)),
            _ => Err(Error::Unexpected),
        }
    }

    #[inline]
    fn decode_slice(&mut self, len: u64) -> Result<&'de [u8]> {
        if self.input.len() < len as usize {
            Err(Error::Decode(DecodeError::Eof))
        } else {
            let tmp = &self.input[.. len as usize];
            self.input = &self.input[len as usize ..];
            Ok(tmp)
        }
    }

}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let (Header(code, value), _) = Header::decode(self.input)?;
        // Don't advance the cursor!
        match code {
            Code::Fixed => {
                match Fixed::from_bits(value)? {
                    Fixed::Unit => visitor.visit_unit(),
                    Fixed::True => visitor.visit_bool(true),
                    Fixed::False => visitor.visit_bool(false),
                    Fixed::F32 => self.deserialize_f32(visitor),
                    Fixed::F64 => self.deserialize_f64(visitor),
                }
            },
            Code::Bytes => self.deserialize_bytes(visitor),
            Code::Intp | Code::Intn => self.deserialize_i64(visitor),
            Code::Str => self.deserialize_str(visitor),
            Code::Container => self.deserialize_seq(visitor),
            Code::Key => self.deserialize_any(visitor), // TODO: nicht ganz akkurat, doppelte Keys sind verboten
            Code::Reserved => Err(Error::Decode(DecodeError::Code(code.to_bits()))),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.decode_fixed()? {
            Fixed::True => visitor.visit_bool(true),
            Fixed::False => visitor.visit_bool(false),
            __ => Err(Error::Unexpected),
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
        match self.decode_fixed()? {
            Fixed::F32 => visitor.visit_f32(<f32>::from_be_bytes(self.decode_slice(4)?.try_into().unwrap())),
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.decode_fixed()? {
            Fixed::F64 => visitor.visit_f64(<f64>::from_be_bytes(self.decode_slice(8)?.try_into().unwrap())),
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let (code, value) = self.decode_header()?;
        match code {
            Code::Str if value == 1 => {
                visitor.visit_char(self.decode_slice(1)?[0] as char)
            }
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let (code, value) = self.decode_header()?;
        match code {
            Code::Str => visitor.visit_borrowed_str(std::str::from_utf8(self.decode_slice(value)?)?),
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let (code, value) = self.decode_header()?;
        match code {
            Code::Bytes => visitor.visit_borrowed_bytes(self.decode_slice(value)?),
            Code::Container => {
                let mut bytes = Vec::<u8>::with_capacity(value as usize);
                for _ in 0..value {
                    bytes.push(self.decode_int()?.try_into()?);
                }
                visitor.visit_byte_buf(bytes)
            },
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let (code, value) = self.decode_header()?;
        match code {
            Code::Bytes => visitor.visit_byte_buf(self.decode_slice(value)?.to_vec()),
            Code::Container => {
                let mut bytes = Vec::with_capacity(value as usize);
                for _ in 0..value {
                    bytes.push(self.decode_int()?.try_into()?);
                }
                visitor.visit_byte_buf(bytes)
            }
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let (Header(code, value), tail) = Header::decode(self.input)?;
        match code {
            Code::Fixed => {
                match Fixed::from_bits(value)? {
                    Fixed::Unit => { 
                        self.input = tail; 
                        visitor.visit_none() 
                    },
                    _ => visitor.visit_some(self),
                }
            },
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.decode_fixed()? {
            Fixed::Unit => visitor.visit_unit(),
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(mut self, visitor: V) -> Result<V::Value> {
        let (code, value) = self.decode_header()?;
        match code {
            Code::Container => visitor.visit_seq(SeqDeserializer::new(&mut self, value)),
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(self, _name: &'static str, _len: usize, visitor: V) -> Result<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(mut self, visitor: V) -> Result<V::Value> {
        let (code, value) = self.decode_header()?;
        match code {
            Code::Container => visitor.visit_map(MapDeserializer::new(&mut self, value)),
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(mut self, _name: &'static str, _fields: &'static [&'static str], visitor: V) -> Result<V::Value> {
        let (code, value) = self.decode_header()?;
        match code {
            Code::Container => visitor.visit_map(StructDeserializer::new(&mut self, value)),
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_enum<V: Visitor<'de>>(self, _name: &'static str, _variants: &'static [&'static str],  visitor: V) -> Result<V::Value> {
        let (code, value) = self.decode_header()?;
        match code {
            Code::Str => visitor.visit_enum(std::str::from_utf8(self.decode_slice(value)?).map_err(|e| <DecodeError>::from(e))?.into_deserializer()),
            Code::Container => visitor.visit_enum(EnumDeserializer::new(&mut *self)),
            _ => Err(Error::Unexpected),
        }
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let (code, value) = self.decode_header()?;
        match code {
            Code::Key => visitor.visit_borrowed_str(std::str::from_utf8(self.decode_slice(value)?).map_err(|e| <DecodeError>::from(e))?),
            _ => Err(Error::Unexpected)
        }
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        // FIXME: wie überspringt man möglichst schnell einen Container?
        let (field, tail) = Field::decode(self.input)?;
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
                Code::Key => {
                    let name = std::str::from_utf8(&tail[..value as usize]).map_err(|e| <DecodeError>::from(e))?; // TODO: panic! im slice access
                    self.de.input = &tail[value as usize ..];
                    seed.deserialize(name.into_deserializer()).map(Some)
                },
                _ => Err(Error::Unexpected)
            }
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        seed.deserialize(&mut *self.de)
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
        Err(Error::Unexpected)
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        seed.deserialize(self.de)
    }

    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value> {
        de::Deserializer::deserialize_seq(self.de, visitor)
    }

    fn struct_variant<V: Visitor<'de>>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value> {
        let (Header(code, value), tail) = Header::decode(self.de.input)?;
        de::Deserializer::deserialize_struct(self.de, "", fields, visitor) // TODO: stimmt das?
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
            self.remaining -= 1;
            seed.deserialize(&mut *self.de).map(Some)
        }
    }

}
