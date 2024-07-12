use serde::{
    de::{Error, Visitor},
    Deserialize, Deserializer,
};

use crate::{Config, Field, FieldType, FieldValue, Metadata, SectionIO, StdError};

// Raw Config
//
// Any serialized config can be deserialized into RawConfig
// Raw config can be deserialized into original config via `deserialize_into_config` function
#[derive(Debug, Clone, Deserialize)]
pub struct RawConfig {
    config_name: String,
    fields: Vec<RawField>,
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
                name: field.key.as_str(),
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
        _value: &FieldValue<'_>,
    ) -> Result<(), crate::StdError> {
        Err("set field value on intermediate config representation is not supported")?
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawField {
    key: String,
    value: RawFieldValue,
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
            RawFieldValue::String(ref v) => v.into(),
            RawFieldValue::Bool(v) => v.into(),
        }
    }
}
struct RawFieldValueVisitor {}

impl<'de> Visitor<'de> for RawFieldValueVisitor {
    type Value = RawFieldValue;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "expected RawFieldValue")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(RawFieldValue::Bool(v))
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(RawFieldValue::I8(v))
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(RawFieldValue::I16(v))
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(RawFieldValue::I32(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(RawFieldValue::I64(v))
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(RawFieldValue::U8(v))
    }

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(RawFieldValue::U16(v))
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(RawFieldValue::U32(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(RawFieldValue::U64(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(RawFieldValue::String(v.into()))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(RawFieldValue::String(v))
    }
}

impl<'de> Deserialize<'de> for RawFieldValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(RawFieldValueVisitor {})
    }
}

enum RawConfigField {
    ConfigName,
    Fields,
    Ignore,
}

impl RawConfigField {
    fn to_str(&self) -> &'static str {
        match self {
            Self::ConfigName => "config_name",
            Self::Fields => "fields",
            Self::Ignore => "",
        }
    }
}

impl<'de> Deserialize<'de> for RawConfigField {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_identifier(RawConfigFieldVisitor {})
    }
}
struct RawConfigFieldVisitor;

impl<'de> Visitor<'de> for RawConfigFieldVisitor {
    type Value = RawConfigField;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "field identifier")
    }

    fn visit_u64<E: Error>(self, value: u64) -> Result<Self::Value, E> {
        match value {
            0 => Ok(RawConfigField::ConfigName),
            1 => Ok(RawConfigField::Fields),
            _ => Ok(RawConfigField::Ignore),
        }
    }

    fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
        match value {
            "config_name" => Ok(RawConfigField::ConfigName),
            "fields" => Ok(RawConfigField::Fields),
            _ => Ok(RawConfigField::Ignore),
        }
    }

    fn visit_bytes<E: Error>(self, value: &[u8]) -> Result<Self::Value, E> {
        match value {
            b"config_name" => Ok(RawConfigField::ConfigName),
            b"fields" => Ok(RawConfigField::Fields),
            _ => Ok(RawConfigField::Ignore),
        }
    }
}

struct RawConfigVisitor {}

impl<'de> Visitor<'de> for RawConfigVisitor {
    type Value = Box<RawConfig>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "expected struct RawConfig")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        {
            let mut config_name: Option<String> = None;
            let mut fields: Option<Vec<RawField>> = None;
            while let Some(key) = map.next_key::<RawConfigField>()? {
                match key {
                    RawConfigField::Ignore => Err(Error::custom("unexpected field"))?,
                    RawConfigField::ConfigName if config_name.is_none() => {
                        config_name = Some(map.next_value()?)
                    }
                    RawConfigField::Fields if fields.is_none() => fields = Some(map.next_value()?),
                    _ => Err(Error::duplicate_field(key.to_str()))?,
                }
            }
            match (config_name, fields) {
                (Some(config_name), Some(fields)) => Ok(Box::new(RawConfig {
                    config_name,
                    fields,
                })),
                (None, _) => Err(Error::missing_field(RawConfigField::ConfigName.to_str())),
                (_, None) => Err(Error::missing_field(RawConfigField::Fields.to_str())),
            }
        }
    }
}

impl<'de> Deserialize<'de> for Box<dyn Config> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(deserializer.deserialize_map(RawConfigVisitor {})?)
    }
}

pub fn deserialize_into_config(from: &dyn Config, to: &mut dyn Config) -> Result<(), StdError> {
    for field in from.fields() {
        to.set_field_value(field.name, &field.value)?;
    }
    Ok(())
}
