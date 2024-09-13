use crate::Result;
use config::{prelude::deserialize_into_config, Config as BaseConfig};
use section::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

pub trait Config<Chan: SectionChannel>: BaseConfig + DynSection<Chan> {
    fn as_dyn_section(&self) -> Box<dyn DynSection<Chan>>;
    fn as_dyn_config_ref(&self) -> &dyn BaseConfig;
    fn as_dyn_config_mut_ref(&mut self) -> &mut dyn BaseConfig;
    fn inner_clone(&self) -> Box<dyn Config<Chan>>;
}

impl<Chan: SectionChannel, T: BaseConfig + DynSection<Chan> + Clone> Config<Chan> for T {
    fn as_dyn_section(&self) -> Box<dyn DynSection<Chan>> {
        Box::new(self.clone())
    }

    fn as_dyn_config_ref(&self) -> &dyn BaseConfig {
        self
    }

    fn as_dyn_config_mut_ref(&mut self) -> &mut dyn BaseConfig {
        self
    }
    
    fn inner_clone(&self) -> Box<dyn Config<Chan>> {
        Box::new(self.clone())
    }
}

impl<Chan: SectionChannel> Clone for Box<dyn Config<Chan>> {
    fn clone(&self) -> Self {
        self.inner_clone()
    }
}

impl<Chan: SectionChannel> PartialEq for dyn Config<Chan> {
    fn eq(&self, other: &Self) -> bool {
        self.as_dyn_config_ref() == other.as_dyn_config_ref()
    }
}

pub type ConfigConstructor<Chan> = fn() -> Box<dyn Config<Chan>>;

impl<Chan: SectionChannel> Serialize for dyn Config<Chan> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s: &dyn BaseConfig = self.as_dyn_config_ref();
        s.serialize(serializer)
    }
}

impl<'de, Chan: SectionChannel> Deserialize<'de> for Box<dyn Config<Chan>> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw_config: Box<dyn BaseConfig> =
            <Box<dyn BaseConfig> as Deserialize>::deserialize(deserializer)?;
        Ok(Box::new(RawConfig(raw_config)))
    }
}

#[derive(Debug, Clone)]
pub struct RawConfig(Box<dyn BaseConfig>);

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for RawConfig {
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(self, _: Input, _: Output, _: SectionChan) -> Self::Future {
        Box::pin(async { Err("attempt to run section on raw config")? })
    }
}

impl BaseConfig for RawConfig {
    fn name(&self) -> &str {
        self.0.name()
    }

    fn input(&self) -> config::SectionIO {
        self.0.input()
    }

    fn output(&self) -> config::SectionIO {
        self.0.output()
    }

    fn fields(&self) -> Vec<config::Field<'_>> {
        self.0.fields()
    }

    fn get_field_value(&self, name: &str) -> Result<config::FieldValue<'_>> {
        self.0.get_field_value(name)
    }

    fn set_field_value(&mut self, name: &str, value: config::FieldValue<'_>) -> Result<()> {
        self.0.set_field_value(name, value)
    }

    fn strip_secrets(&mut self) {
        self.0.strip_secrets()
    }

    fn clone_config(&self) -> Box<dyn config::Config> {
        self.0.clone_config()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigMetaData<Chan> {
    pub input: bool,
    pub output: bool,
    pub ty: Arc<str>,
    ctor: ConfigConstructor<Chan>,
}

impl<Chan> ConfigMetaData<Chan> {
    pub fn build_config(&self) -> Box<dyn Config<Chan>> {
        (self.ctor)()
    }
}

#[derive(Debug)]
pub struct ConfigRegistry<Chan = section::dummy::DummySectionChannel> {
    registry: BTreeMap<Arc<str>, ConfigMetaData<Chan>>,
}

impl<Chan: SectionChannel> Default for ConfigRegistry<Chan> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Chan: SectionChannel> ConfigRegistry<Chan> {
    pub fn new() -> Self {
        Self {
            registry: BTreeMap::new(),
        }
    }

    pub fn add_config(&mut self, ctor: ConfigConstructor<Chan>) -> Result<()> {
        let config = ctor();
        let name: Arc<str> = Arc::from(config.name());
        let (input, output) = (!config.input().is_none(), !config.output().is_none());
        let metadata = ConfigMetaData {
            input,
            output,
            ty: Arc::clone(&name),
            ctor,
        };
        if self.registry.contains_key(&*name) {
            Err(format!("{name} already present"))?
        };
        self.registry.insert(Arc::clone(&name), metadata);
        Ok(())
    }

    pub fn build_config(&self, name: &str) -> Result<Box<dyn Config<Chan>>> {
        match self.registry.get(name) {
            Some(metadata) => Ok(metadata.build_config()),
            None => Err(format!("no config constructor for {name} found"))?,
        }
    }

    // deserialize raw config into real config
    pub fn deserialize_config(
        &self,
        raw_config: &dyn Config<Chan>,
    ) -> Result<Box<dyn Config<Chan>>> {
        let mut config = self.build_config(raw_config.name())?;
        deserialize_into_config(
            raw_config.as_dyn_config_ref(),
            config.as_dyn_config_mut_ref(),
        )?;
        Ok(config)
    }
}
