use serde::Deserialize;

use crate::{Config, Field, FieldType, FieldValue, Metadata, SectionIO};

pub(crate) mod de;
pub(crate) mod ser;

// Raw Config
//
// Any serialized config can be deserialized into RawConfig
// Raw config can be deserialized into original config via `deserialize_into_config` function
#[derive(Debug, Clone, Deserialize)]
pub struct RawConfig {
    config_name: String,
    fields: Vec<RawField>,
}

impl RawConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            config_name: name.into(),
            fields: vec![],
        }
    }

    pub fn with_fields<'a>(self, iter: impl Iterator<Item = Field<'a>>) -> Self {
        let Self {
            config_name,
            mut fields,
        } = self;
        for field in iter {
            fields.push(field.into())
        }
        Self {
            config_name,
            fields,
        }
    }
}

impl Config for RawConfig {
    fn name(&self) -> &str {
        self.config_name.as_str()
    }

    fn input(&self) -> SectionIO {
        SectionIO::None
    }

    fn output(&self) -> SectionIO {
        SectionIO::None
    }

    fn fields(&self) -> Vec<Field> {
        self.fields
            .iter()
            .map(|field| Field {
                name: field.name.as_str(),
                ty: FieldType::String,
                metadata: Metadata {
                    is_read_only: true,
                    ..Default::default()
                },
                value: (&field.value).into(),
            })
            .collect()
    }

    fn get_field_value(&self, name: &str) -> Result<FieldValue<'_>, crate::StdError> {
        match self.fields().into_iter().find(|field| field.name == name) {
            Some(field) => Ok(field.value),
            None => Err(format!("unmatched field name '{name}'"))?,
        }
    }

    fn set_field_value(
        &mut self,
        _name: &str,
        _value: FieldValue<'_>,
    ) -> Result<(), crate::StdError> {
        Err("set field value on intermediate config representation is not supported")?
    }

    fn strip_secrets(&mut self) {
        // raw config doesn't have any metadata
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawField {
    name: String,
    value: RawFieldValue,
}

impl From<Field<'_>> for RawField {
    fn from(field: Field<'_>) -> Self {
        RawField {
            name: field.name.into(),
            value: field.value.into(),
        }
    }
}

#[derive(Debug, Clone)]
enum RawFieldValue {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    String(String),
    Bool(bool),
}

impl From<FieldValue<'_>> for RawFieldValue {
    fn from(value: FieldValue<'_>) -> Self {
        match value {
            FieldValue::I8(v) => RawFieldValue::I8(v),
            FieldValue::I16(v) => RawFieldValue::I16(v),
            FieldValue::I32(v) => RawFieldValue::I32(v),
            FieldValue::I64(v) => RawFieldValue::I64(v),
            FieldValue::U8(v) => RawFieldValue::U8(v),
            FieldValue::U16(v) => RawFieldValue::U16(v),
            FieldValue::U32(v) => RawFieldValue::U32(v),
            FieldValue::U64(v) => RawFieldValue::U64(v),
            FieldValue::String(v) => RawFieldValue::String(v.into()),
            FieldValue::Bool(v) => RawFieldValue::Bool(v),
        }
    }
}

impl<'a> From<&'a RawFieldValue> for FieldValue<'a> {
    fn from(value: &'a RawFieldValue) -> FieldValue<'a> {
        match value {
            RawFieldValue::I8(v) => v.into(),
            RawFieldValue::I16(v) => v.into(),
            RawFieldValue::I32(v) => v.into(),
            RawFieldValue::I64(v) => v.into(),
            RawFieldValue::U8(v) => v.into(),
            RawFieldValue::U16(v) => v.into(),
            RawFieldValue::U32(v) => v.into(),
            RawFieldValue::U64(v) => v.into(),
            RawFieldValue::String(v) => v.into(),
            RawFieldValue::Bool(v) => v.into(),
        }
    }
}
