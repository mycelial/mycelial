use std::collections::BTreeMap;
use std::sync::Arc;

type Result<T, E = Box<dyn std::error::Error + Send + Sync + 'static>> = std::result::Result<T, E>;

pub type ConfigConstructor = fn() -> Box<dyn config::Config>;

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigMetaData {
    pub input: bool,
    pub output: bool,
    pub ty: Arc<str>,
    ctor: ConfigConstructor,
}

impl ConfigMetaData {
    pub fn build_config(&self) -> Box<dyn config::Config> {
        (self.ctor)()
    }
}

#[derive(Debug)]
pub struct ConfigRegistry {
    registry: BTreeMap<Arc<str>, ConfigMetaData>,
}

impl Default for ConfigRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigRegistry {
    pub fn new() -> Self {
        Self {
            registry: BTreeMap::new(),
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

    pub fn build_config(&self, name: &str) -> Result<Box<dyn config::Config>> {
        match self.registry.get(name) {
            Some(metadata) => Ok(metadata.build_config()),
            None => Err(format!("no config constructor for {name} found"))?,
        }
    }
}

pub fn new() -> Result<ConfigRegistry> {
    let mut registry = ConfigRegistry::new();
    registry.add_config(|| Box::from(csv_transform::FromCsv::default()))?;
    registry.add_config(|| Box::from(csv_transform::ToCsv::default()))?;
    Ok(registry)
}
