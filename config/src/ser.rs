use serde::{ser::SerializeMap, Serialize};

use crate::{Config, Field, FieldValue};

#[repr(transparent)]
struct Slice<'a> (&'a [Field<'a>]);

impl Serialize for dyn Config {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
        where
            S: serde::Serializer
    {
        // FIXME: for now only support structs
        let mut top_level_struct = serializer.serialize_map(Some(1))?;
        top_level_struct.serialize_entry(self.name(), &Slice(self.fields().as_slice()))?;
        top_level_struct.end()
    }
}

impl Serialize for Slice<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: serde::Serializer
    {
        let mut inner_map= serializer.serialize_map(Some(self.0.len()))?;
        for field in self.0 {
            match &field.value {
                FieldValue::I8(v) => inner_map.serialize_entry(field.name, &v)?,
                FieldValue::I16(v) => inner_map.serialize_entry(field.name, &v)?,
                FieldValue::I32(v) => inner_map.serialize_entry(field.name, &v)?,
                FieldValue::I64(v) => inner_map.serialize_entry(field.name, &v)?,
                FieldValue::U8(v) => inner_map.serialize_entry(field.name, &v)?,
                FieldValue::U16(v) => inner_map.serialize_entry(field.name, &v)?,
                FieldValue::U32(v) => inner_map.serialize_entry(field.name, &v)?,
                FieldValue::U64(v) => inner_map.serialize_entry(field.name, &v)?,
                FieldValue::Bool(v) => inner_map.serialize_entry(field.name, &v)?,
                FieldValue::String(v) => inner_map.serialize_entry(field.name, &v)?,
                FieldValue::Vec(v) => {
                    unimplemented!("serialization for vector values are not yet implemented")
                }
            };
        };
        inner_map.end()
    }
}