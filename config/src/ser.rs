use serde::{
    ser::{SerializeMap, SerializeSeq},
    Serialize,
};

use crate::{Config, Field, FieldValue};

#[repr(transparent)]
struct Slice<'a>(&'a [Field<'a>]);

impl Serialize for dyn Config {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // FIXME: for now only support structs
        let mut top_level_struct = serializer.serialize_map(Some(2))?;
        top_level_struct.serialize_entry("config_name", self.name())?;
        top_level_struct.serialize_entry("fields", &Slice(self.fields().as_slice()))?;
        top_level_struct.end()
    }
}

impl Serialize for Slice<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for field in self.0 {
            seq.serialize_element(&field)?;
        }
        seq.end()
    }
}

impl<'a> Serialize for &'a Field<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("key", self.name)?;
        match &self.value {
            FieldValue::I8(v) => map.serialize_entry("value", &v)?,
            FieldValue::I16(v) => map.serialize_entry("value", &v)?,
            FieldValue::I32(v) => map.serialize_entry("value", &v)?,
            FieldValue::I64(v) => map.serialize_entry("value", &v)?,
            FieldValue::U8(v) => map.serialize_entry("value", &v)?,
            FieldValue::U16(v) => map.serialize_entry("value", &v)?,
            FieldValue::U32(v) => map.serialize_entry("value", &v)?,
            FieldValue::U64(v) => map.serialize_entry("value", &v)?,
            FieldValue::Bool(v) => map.serialize_entry("value", &v)?,
            FieldValue::String(v) => map.serialize_entry("value", &v)?,
        };
        map.end()
    }
}
