use serde::ser::{self, Serialize};
use nachricht::{EncodeError, Header, Sign};
use std::io::Write;
use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::preser::{Layout, Layouts, preserialize};

pub struct Serializer<W> {
    layouts: Layouts,
    symbols: HashMap<&'static str, usize>,
    next_free: usize,
    output: W,
}

pub fn to_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut serializer = Serializer {
        output: Vec::new(),
        symbols: HashMap::new(),
        layouts: preserialize(value)?,
        next_free: 0
    };
    value.serialize(&mut serializer)?;
    Ok(serializer.output())
}

pub fn to_writer<T: Serialize, W: Write>(writer: W, value: &T) -> Result<()> {
    let mut serializer = Serializer {
        output: writer,
        symbols: HashMap::new(),
        layouts: preserialize(value)?,
        next_free: 0
    };
    value.serialize(&mut serializer)?;
    Ok(())
}

impl Serializer<Vec<u8>> {
    fn output(self) -> Vec<u8> {
        self.output
    }
}

impl<W: Write> Serializer<W> {

    fn next(&mut self) -> usize {
        self.next_free += 1;
        self.next_free - 1
    }

    #[inline(always)]
    fn get_variant_idx(&mut self, name: &'static str, variant: &'static str) -> Result<&mut Option<usize>> {
        Ok(self.layouts.variants.get_mut(name).and_then(|m| m.get_mut(variant)).ok_or(Error::UnknownVariantLayout(name, variant))?)
    }

    #[inline(always)]
    fn get_layout(&mut self, name: &'static str, variant: Option<&'static str>) -> Result<&mut Layout> {
        Ok(self.layouts.structs.get_mut(name).and_then(|m| m.get_mut(&variant)).ok_or(Error::UnknownStructLayout(name))?)
    }

    fn serialize_symbol(&mut self, symbol: &'static str) -> Result<()> {
        match self.symbols.get(symbol) {
            Some(i) => { Header::Ref(*i).encode(&mut self.output)?; },
            None    => {
                Header::Sym(symbol.len()).encode(&mut self.output)?;
                self.output.write_all(symbol.as_bytes()).map_err(EncodeError::from)?;
                let next = self.next();
                self.symbols.insert(symbol, next);
            }
        }
        Ok(())
    }

    fn serialize_layout(&mut self, name: &'static str, variant: Option<&'static str>) -> Result<()> {
        let layout = self.get_layout(name, variant)?;
        let fields = layout.fields.clone();
        match layout.idx {
            Some(i) => { Header::Ref(i).encode(&mut self.output)?; },
            None    => {
                Header::Rec(fields.len()).encode(&mut self.output)?;
                for sym in fields.iter() {
                    self.serialize_symbol(sym)?;
                }
                let next = self.next();
                self.get_layout(name, variant)?.idx.replace(next);
            }
        };
        Ok(())
    }

    fn serialize_variant(&mut self, name: &'static str, variant: &'static str) -> Result<()> {
        let idx = self.get_variant_idx(name, variant)?;
        match idx {
            Some(i) => { Header::Ref(*i).encode(&mut self.output)?; },
            None    => {
                Header::Rec(1).encode(&mut self.output)?;
                self.serialize_symbol(variant)?;
                let next = self.next();
                self.get_variant_idx(name, variant)?.replace(next);
            }
        };
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
        (if v { Header::True } else { Header::False }).encode(&mut self.output)?;
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
        Header::Int(if v < 0 { Sign::Neg } else { Sign::Pos }, v.unsigned_abs()).encode(&mut self.output)?;
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
        Header::Int(Sign::Pos, v).encode(&mut self.output)?;
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
        self.serialize_symbol(variant)
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _name: &'static str, value: &T) -> Result<()> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(self, name: &'static str, _index: u32, variant: &'static str, value: &T) -> Result<()> {
        self.serialize_variant(name, variant)?;
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        match len {
            Some(l) => {
                Header::Arr(l).encode(&mut self.output)?;
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

    fn serialize_tuple_variant(self, name: &'static str, _index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeTupleVariant> {
        self.serialize_variant(name, variant)?;
        Header::Arr(len).encode(&mut self.output)?;
        Ok(self)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        match len {
            Some(len) => {
                Header::Map(len).encode(&mut self.output)?;
                Ok(self)
            },
            None => Err(Error::Length)
        }
    }

    fn serialize_struct(self, name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_layout(name, None)?;
        Ok(self)
    }

    fn serialize_struct_variant(self, name: &'static str, _index: u32, variant: &'static str, _len: usize) -> Result<Self::SerializeStructVariant> {
        self.serialize_variant(name, variant)?;
        self.serialize_layout(name, Some(variant))?;
        Ok(self)
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

impl<'a, W: Write> ser::SerializeStructVariant for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, _key: &'static str, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }

}

impl<'a, W: Write> ser::SerializeStruct for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, _key: &'static str, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }

}
