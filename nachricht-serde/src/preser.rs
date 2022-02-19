use serde::ser::{self, Serialize};
use std::collections::HashMap;

use crate::error::{Error, Result};

/// For enum identifiers: name => variant => T
/// Structs don't have variants, hence the second parameter is optional
pub type Variant<T> = HashMap<&'static str, HashMap<Option<&'static str>, T>>;

#[derive(Default,Debug)]
pub struct Layout {
    pub fields: Vec<&'static str>,
    pub idx: Option<usize>,
}

impl Layout {
    fn from(fields: Vec<&'static str>) -> Self {
        Self { fields, idx: None }
    }
}

#[derive(Default,Debug)]
pub struct Layouts {
    /// The name of the variant already defines the layout of the used record, hence we only have to
    /// track the index
    pub variants: HashMap<&'static str, HashMap<&'static str, Option<usize>>>,
    pub structs: Variant<Layout>,
}

pub struct Preserializer {
    layouts: Layouts,
}

pub fn preserialize<T: Serialize>(value: &T) -> Result<Layouts> {
    let mut preserializer = Preserializer { layouts: Default::default() };
    value.serialize(&mut preserializer)?;
    Ok(preserializer.layouts)
}

impl Preserializer {

    fn add_struct_layout(&mut self, name: &'static str, variant: Option<&'static str>, layout: Vec<&'static str>) -> Result<()> {
        match self.layouts.structs.entry(name).or_default().insert(variant, Layout::from(layout.clone())) {
            Some(old) if old.fields != *layout => Err(Error::DuplicateLayout(name, variant)),
            _ => Ok(())
        }
    }

    fn add_variant(&mut self, name: &'static str, variant: &'static str) {
        self.layouts.variants.entry(name).or_default().insert(variant, None);
    }

}

impl<'a> ser::Serializer for &'a mut Preserializer {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = StructPreserializer<'a>;
    type SerializeStructVariant = VariantPreserializer<'a>;

    fn serialize_bool(self, _v: bool) -> Result<()> {
        Ok(())
    }
    
    fn serialize_i8(self, _v: i8) -> Result<()> {
        Ok(())
    }

    fn serialize_i16(self, _v: i16) -> Result<()> {
        Ok(())
    }

    fn serialize_i32(self, _v: i32) -> Result<()> {
        Ok(())
    }

    fn serialize_i64(self, _v: i64) -> Result<()> {
        Ok(())
    }

    fn serialize_u8(self, _v: u8) -> Result<()> {
        Ok(())
    }

    fn serialize_u16(self, _v: u16) -> Result<()> {
        Ok(())
    }

    fn serialize_u32(self, _v: u32) -> Result<()> {
        Ok(())
    }

    fn serialize_u64(self, _v: u64) -> Result<()> {
        Ok(())
    }

    fn serialize_f32(self, _v: f32) -> Result<()> {
        Ok(())
    }

    fn serialize_f64(self, _v: f64) -> Result<()> {
        Ok(())
    }

    fn serialize_char(self, _v: char) -> Result<()> {
        Ok(())
    }

    fn serialize_str(self, _v: &str) -> Result<()> {
        Ok(())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
        Ok(())
    }

    fn serialize_none(self) -> Result<()> {
        Ok(())
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<()> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_variant(self, _name: &'static str, _index: u32, _variant: &'static str) -> Result<()> {
        Ok(())
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _name: &'static str, value: &T) -> Result<()> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(self, name: &'static str, _index: u32, variant: &'static str, value: &T) -> Result<()> {
        self.add_variant(name, variant);
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(self)
    }

    fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeTupleStruct> {
        Ok(self)
    }

    fn serialize_tuple_variant(self, name: &'static str, _index: u32, variant: &'static str, _len: usize) -> Result<Self::SerializeTupleVariant> {
        self.add_variant(name, variant);
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(self)
    }

    fn serialize_struct(self, name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(StructPreserializer { ser: self, name, fields: Vec::new() })
    }

    fn serialize_struct_variant(self, name: &'static str, _index: u32, variant: &'static str, _len: usize) -> Result<Self::SerializeStructVariant> {
        self.add_variant(name, variant);
        Ok(VariantPreserializer { ser: self, variant, name, fields: Vec::new() })
    }

}

impl ser::SerializeSeq for &mut Preserializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }

}

impl ser::SerializeTuple for &mut Preserializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl ser::SerializeTupleStruct for &mut Preserializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl ser::SerializeTupleVariant for &mut Preserializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl ser::SerializeMap for &mut Preserializer {
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

pub struct StructPreserializer<'a> {
    ser: &'a mut Preserializer,
    name: &'static str,
    fields: Vec<&'static str>,
}

impl<'a> ser::SerializeStruct for StructPreserializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<()> {
        self.fields.push(key);
        value.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn end(mut self) -> Result<()> {
        self.ser.add_struct_layout(self.name, None, self.fields.drain(..).collect())?;
        Ok(())
    }

}

pub struct VariantPreserializer<'a> {
    ser: &'a mut Preserializer,
    name: &'static str,
    variant: &'static str,
    fields: Vec<&'static str>,
}

impl<'a> ser::SerializeStructVariant for VariantPreserializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<()> {
        self.fields.push(key);
        value.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn end(mut self) -> Result<()> {
        self.ser.add_struct_layout(self.name, Some(self.variant), self.fields.drain(..).collect())?;
        Ok(())
    }

}
