use serde::{ser, Serialize};
use nachricht::{Value, Field, Sign, Header, Code, EncodeError};
use std::io::Write;

use crate::error::{Error, Result};

enum Style {
    /// Better if schema evolution is likely and interop necessary
    Named,
    /// More compact, usable if schema evolution is unlikely or interop not a requirement
    Unnamed,
}

pub struct Serializer {
    style: Style,
    output: Vec<u8>,
}

pub fn to_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut serializer = Serializer { output: Vec::new(), style: Style::Named }; // TODO: to_bytes und to_bytes_named
    value.serialize(&mut serializer)?;
    Ok(serializer.output)
}

// TODO: to_writer

impl<'a> ser::Serializer for &'a mut Serializer {

    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        Field { name: None, value: Value::Bool(v) }.encode(&mut self.output)?;
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        Field { name: None, value: Value::Int(if v < 0 { Sign::Neg } else { Sign::Pos }, v.abs() as u64) }.encode(&mut self.output)?;
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        Field { name: None, value: Value::Int(Sign::Pos, v) }.encode(&mut self.output)?;
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        Field { name: None, value: Value::F32(v) }.encode(&mut self.output)?;
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        Field { name: None, value: Value::F64(v) }.encode(&mut self.output)?;
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        Field { name: None, value: Value::Str(v) }.encode(&mut self.output)?;
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        Field { name: None, value: Value::Bytes(v) }.encode(&mut self.output)?;
        Ok(())
    }

    fn serialize_none(self) -> Result<()> {
        Field { name: None, value: Value::Unit }.encode(&mut self.output)?;
        Ok(())
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<()> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        Field { name: None, value: Value::Unit }.encode(&mut self.output)?;
        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(self, _name: &'static str, index: u32, variant: &'static str) -> Result<()> {
        match self.style {
            Style::Named => Field { name: None, value: Value::Str(variant) }.encode(&mut self.output)?,
            Style::Unnamed => Field { name: None, value: Value::Int(Sign::Pos, index as u64) }.encode(&mut self.output)?,
        };
        Ok(())
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _name: &'static str, value: &T) -> Result<()> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(self, name: &'static str, index: u32, variant: &'static str, value: &T) -> Result<()> {
        Field { name: None, value: Value::Container(vec![Field { name: Some(name), value: Value::Str(variant)}])}.encode(&mut self.output)?;
        Ok(())
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        match len {
            Some(l) => { 
                Header(Code::Container, l as u64).encode(&mut self.output)?;
                Ok(self)
            },
            None => Err(Error::Length),
        }
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(self, name: &'static str, index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeTupleVariant> {
        Header(Code::Container, 1).encode(&mut self.output)?;
        Header(Code::Key, variant.len() as u64).encode(&mut self.output)?;
        self.output.write_all(variant.as_bytes()).map_err(|e| EncodeError::Io(e))?;
        Header(Code::Container, len as u64).encode(&mut self.output)?;
        Ok(self)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        match len {
            Some(len) => {
                Header(Code::Container, (len as u64) << 1).encode(&mut self.output)?;
                Ok(self)
            },
            None => Err(Error::Length)
        }
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(self, name: &'static str, index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeStructVariant> {
        self.serialize_tuple_variant(name, index, variant, len)
    }

}

impl<'a> ser::SerializeSeq for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }

}

impl<'a> ser::SerializeTuple for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<()> {
        key.serialize(&mut **self)
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }

}

impl<'a> ser::SerializeStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<()> {
        Header(Code::Key, key.len() as u64).encode(&mut self.output)?;
        self.output.write_all(key.as_bytes()).map_err(|e| EncodeError::Io(e))?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }

}

impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<()> {
        Header(Code::Key, key.len() as u64).encode(&mut self.output)?;
        self.output.write_all(key.as_bytes()).map_err(|e| EncodeError::Io(e))?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }

}
