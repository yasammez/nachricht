use serde::{ser, Serialize};
use nachricht::{EncodeError, Header, Refable};
use std::io::Write;

use crate::error::{Error, Result};

pub struct Serializer<W> {
    symbols: Vec<(Refable, String)>,
    output: W,
}

pub fn to_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let buf = Vec::new();
    let mut serializer = Serializer { output: buf, symbols: Vec::new() };
    value.serialize(&mut serializer)?;
    Ok(serializer.output())
}

pub fn to_writer<T: Serialize, W: Write>(writer: W, value: &T) -> Result<()> {
    let mut serializer = Serializer { output: writer, symbols: Vec::new() };
    value.serialize(&mut serializer)?;
    Ok(())
}

impl Serializer<Vec<u8>> {
    fn output(self) -> Vec<u8> {
        self.output
    }
}

impl<W: Write> Serializer<W> {
    fn serialize_refable(&mut self, key: &str, kind: Refable) -> Result<()> {
        match self.symbols.iter().enumerate().find(|(_, (k, v))| *k == kind && v == key) {
            Some((i, _)) => { Header::Ref(i).encode(&mut self.output)?; },
            None         => {
                self.symbols.push((kind, key.to_owned()));
                match kind { Refable::Key => Header::Key(key.len()), Refable::Sym => Header::Sym(key.len()) }.encode(&mut self.output)?;
                self.output.write_all(key.as_bytes()).map_err(EncodeError::from)?;
            }
        }
        Ok(())
    }
}

impl<'a, W: Write> ser::Serializer for &'a mut Serializer<W> {

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
        match v { true => Header::True, false => Header::False }.encode(&mut self.output)?;
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
        if v < 0 {
            Header::Neg(v.abs() as u64)
        } else {
            Header::Pos(v as u64)
        }.encode(&mut self.output)?;
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
        Header::Pos(v).encode(&mut self.output)?;
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        Header::F32.encode(&mut self.output)?;
        self.output.write_all(&v.to_be_bytes()).map_err(EncodeError::from)?;
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        Header::F64.encode(&mut self.output)?;
        self.output.write_all(&v.to_be_bytes()).map_err(EncodeError::from)?;
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        Header::Str(v.len()).encode(&mut self.output)?;
        self.output.write_all(v.as_bytes()).map_err(EncodeError::from)?;
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        Header::Bin(v.len()).encode(&mut self.output)?;
        self.output.write_all(v).map_err(EncodeError::from)?;
        Ok(())
    }

    fn serialize_none(self) -> Result<()> {
        Header::Null.encode(&mut self.output)?;
        Ok(())
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<()> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        Header::Null.encode(&mut self.output)?;
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(self, _name: &'static str, _index: u32, variant: &'static str) -> Result<()> {
       self.serialize_refable(variant, Refable::Sym)
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _name: &'static str, value: &T) -> Result<()> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(self, _name: &'static str, _index: u32, variant: &'static str, value: &T) -> Result<()> {
        Header::Bag(1).encode(&mut self.output)?;
        self.serialize_refable(variant, Refable::Key)?;
        value.serialize(self)?;
        Ok(())
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        match len {
            Some(l) => {
                Header::Bag(l).encode(&mut self.output)?;
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

    fn serialize_tuple_variant(self, _name: &'static str, _index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeTupleVariant> {
        Header::Bag(1).encode(&mut self.output)?;
        self.serialize_refable(variant, Refable::Key)?;
        Header::Bag(len).encode(&mut self.output)?;
        Ok(self)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        match len {
            Some(len) if len > usize::MAX >> 1 => Err(Error::MapSize(len)),
            Some(len) => {
                Header::Bag(len << 1).encode(&mut self.output)?;
                Ok(self)
            },
            None => Err(Error::Length)
        }
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        Header::Bag(len).encode(&mut self.output)?;
        Ok(self)
    }

    fn serialize_struct_variant(self, name: &'static str, index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeStructVariant> {
        self.serialize_tuple_variant(name, index, variant, len)
    }

}

impl<'a, W: Write> ser::SerializeSeq for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }

}

impl<'a, W: Write> ser::SerializeTuple for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, W: Write> ser::SerializeTupleStruct for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, W: Write> ser::SerializeTupleVariant for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, W: Write> ser::SerializeMap for &'a mut Serializer<W> {
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

impl<'a, W: Write> ser::SerializeStruct for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<()> {
        self.serialize_refable(key, Refable::Key)?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }

}

impl<'a, W: Write> ser::SerializeStructVariant for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<()> {
        self.serialize_refable(key, Refable::Key)?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }

}
