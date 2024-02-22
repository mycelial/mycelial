//! Dynamic section configuration
use section::SectionError;
use std::collections::HashMap;

pub type Map = HashMap<String, Value>;

/// Pipe Config
///
/// Pipe represented as a vector of sections.
/// Section configuration is a dynamic thing and represented by Map
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    sections: Vec<Map>,
}

/// config Value
///
/// Minimal enum implementation which allows to wrap toml::Value or serde_json::Value
/// To be extended as required
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Map(Map),
    Array(Vec<Value>),
    String(String),
    Bool(bool),
    Int(i64),
}

impl Value {
    pub fn get(&self, key: impl AsRef<str>) -> Option<&Value> {
        match self {
            Self::Map(ref m) => m.get(key.as_ref()),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Self::Array(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&HashMap<String, Value>> {
        match self {
            Self::Map(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            _ => None,
        }
    }
}

impl TryFrom<toml::Value> for Value {
    type Error = SectionError;

    fn try_from(value: toml::Value) -> Result<Self, Self::Error> {
        let value = match value {
            toml::Value::String(s) => Value::String(s),
            toml::Value::Integer(i) => Value::Int(i),
            toml::Value::Boolean(b) => Value::Bool(b),
            toml::Value::Array(v) => Value::Array(
                v.into_iter()
                    .map(Self::try_from)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            toml::Value::Table(m) => Value::Map(
                m.into_iter()
                    .map(|(key, value)| match Self::try_from(value) {
                        Ok(v) => Ok((key, v)),
                        Err(e) => Err(e),
                    })
                    .collect::<Result<HashMap<_, _>, _>>()?,
            ),
            _ => return Err(format!("unsupported value type {value:?}").into()),
        };
        Ok(value)
    }
}

impl TryFrom<serde_json::Value> for Value {
    type Error = SectionError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        let value = match value {
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Number(i) if i.is_i64() => Value::Int(i.as_i64().unwrap()),
            serde_json::Value::Bool(b) => Value::Bool(b),
            serde_json::Value::Array(v) => Value::Array(
                v.into_iter()
                    .map(Self::try_from)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            serde_json::Value::Object(m) => Value::Map(
                m.into_iter()
                    .map(|(key, value)| match Self::try_from(value) {
                        Ok(v) => Ok((key, v)),
                        Err(e) => Err(e),
                    })
                    .collect::<Result<HashMap<_, _>, _>>()?,
            ),
            _ => return Err(format!("unsupported value type {value:?}").into()),
        };
        Ok(value)
    }
}

impl TryFrom<Value> for Config {
    type Error = SectionError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Array(_) => (),
            _ => return Err(format!("expected Value::Array, got: {:?}", value).into()),
        };
        let sections = value
            .as_array()
            .ok_or::<SectionError>("pipe value should be defined as an array".into())?;
        let sections = sections
            .iter()
            .map(|section_cfg| {
                section_cfg
                    .as_map()
                    .map(Clone::clone)
                    .ok_or::<SectionError>(
                        "pipe section configuration should be of map type".into(),
                    )
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { sections })
    }
}

impl Config {
    pub fn new(sections: Vec<Map>) -> Self {
        Self { sections }
    }

    pub fn get_sections(&self) -> &[HashMap<String, Value>] {
        self.sections.as_slice()
    }

    pub fn try_from_json(s: &str) -> Result<Self, SectionError> {
        let value: serde_json::Value = s.parse()?;
        let value: Value = value.try_into()?;
        Self::try_from(value)
    }

    pub fn try_from_toml(s: &str) -> Result<Self, SectionError> {
        let value: toml::Value = s.parse()?;
        let value: Value = Value::try_from(value)?;
        let value = match value {
            Value::Map(mut map) => match map.remove("section") {
                Some(v) => v,
                None => Value::Array(vec![]),
            },
            _ => Err(format!("unsupported value: {:?}", value))?,
        };
        Self::try_from(value)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_configs() {
        let toml = r#"
            [[section]]
            name = "name"
            key = "key"

            [[section]]
            name = "name2"
            value = "value"
        "#;

        let json = r#"
            [
                {"name": "name", "key": "key"},
                {"name": "name2", "value": "value"}
            ]
        "#;
        let json_config = Config::try_from_json(json).unwrap();
        let toml_config = Config::try_from_toml(toml).unwrap();
        assert_eq!(toml_config, json_config);
    }
}
