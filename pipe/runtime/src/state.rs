//! Section state

use super::types::SectionError;

/// FIXME: is there a way to avoid dependency on serde_json?
/// erased-serde can be used for dyn Serialize/Deserialize types
#[derive(Debug, Clone)]
pub struct State(serde_json::Map<String, serde_json::Value>);

impl State {
    pub fn new() -> Self {
        Self(Default::default())
    }

    pub fn upsert(&mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) {
        self.0.insert(key.into(), value.into());
    }

    pub fn delete(&mut self, key: impl AsRef<str>) {
        self.0.remove(key.as_ref());
    }

    pub fn get(&self, key: impl AsRef<str>) -> Option<serde_json::Value> {
        self.0.get(key.as_ref()).map(Clone::clone)
    }

    pub fn serialize(&self) -> Result<String, SectionError> {
        Ok(serde_json::to_string(&self.0)?)
    }

    pub fn deserialize(state: &str) -> Result<Self, SectionError> {
        match serde_json::from_str(state)? {
            serde_json::Value::Object(map) => Ok(Self(map)),
            _ => Err("invalid state")?,
        }
    }

    pub fn into_inner(self) -> serde_json::Map<String, serde_json::Value> {
        self.0
    }
}
