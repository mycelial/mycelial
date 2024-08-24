use crate::Result;
pub use config::Config;
use section::{dummy::DummySectionChannel, prelude::*};
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::sync::Arc;

pub type ConfigConstructor = fn() -> Box<dyn Config>;

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigMetaData {
    pub input: bool,
    pub output: bool,
    pub ty: Arc<str>,
    ctor: ConfigConstructor,
}

impl ConfigMetaData {
    pub fn build_config(&self) -> Box<dyn Config> {
        (self.ctor)()
    }
}

#[derive(Debug)]
pub struct ConfigRegistry<Chan = DummySectionChannel>
where
    Chan: SectionChannel,
{
    registry: BTreeMap<Arc<str>, ConfigMetaData>,
    _marker: PhantomData<Chan>,
}

impl<Chan: SectionChannel> ConfigRegistry<Chan> {
    pub fn new() -> Self {
        Self {
            registry: BTreeMap::new(),
            _marker: PhantomData,
        }
    }

    pub fn add_config(&mut self, ctor: ConfigConstructor) -> Result<()> {
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

    pub fn iter_values(&self) -> impl Iterator<Item = ConfigMetaData> + '_ {
        self.registry.values().cloned()
    }

    pub fn build_config(&self, name: &str) -> Result<Box<dyn Config>> {
        match self.registry.get(name) {
            Some(metadata) => Ok(metadata.build_config()),
            None => Err(format!("no config constructor for {name} found"))?,
        }
    }
}
