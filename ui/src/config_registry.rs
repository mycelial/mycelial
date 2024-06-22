use std::{collections::BTreeMap, rc::Rc};

use crate::Result;
use config::prelude::*;

#[derive(Debug, Default, Config)]
#[section(output=dataframe)]
pub struct ConfigExample {
    host: String,
    #[validate(min = 1)]
    port: u16,
    user: String,
    #[field_type(password)]
    password: String,
    database: String,
    #[field_type(text_area)]
    query: String,
}

#[derive(Debug, Default, Config)]
#[section(input=dataframe)]
pub struct ConfigExampleSecond {
    host: String,
    #[field_type(password)]
    password: String,
    database: String,
    truncate: bool,
}

pub type ConfigConstructor = fn() -> Box<dyn config::Config>;

#[derive(Debug)]
pub struct ConfigRegistry {
    registry: BTreeMap<Rc<str>, ConfigMetaData>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigMetaData {
    pub input: bool,
    pub output: bool,
    pub ty: Rc<str>,
    ctor: ConfigConstructor,
}

impl ConfigMetaData {
    pub fn build_config(&self) -> Box<dyn config::Config> {
        (self.ctor)()
    }
}

impl Default for ConfigRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigRegistry {
    pub fn new() -> Self {
        let mut s = Self {
            registry: BTreeMap::new(),
        };
        s.add_config(|| Box::from(ConfigExample::default()))
            .unwrap();
        s.add_config(|| Box::from(ConfigExampleSecond::default()))
            .unwrap();
        s
    }

    pub fn add_config(&mut self, ctor: ConfigConstructor) -> Result<()> {
        let config = ctor();
        let name: Rc<str> = Rc::from(config.name());
        let (input, output) = (config.input().is_none(), config.output().is_none());
        let metadata = ConfigMetaData {
            input,
            output,
            ty: Rc::clone(&name),
            ctor,
        };
        if self.registry.get(&*name).is_some() {
            Err(format!("{name} already present"))?
        };
        self.registry.insert(Rc::clone(&name), metadata);
        Ok(())
    }

    pub fn menu_items(&self) -> impl Iterator<Item = ConfigMetaData> + '_ {
        self.registry.values().cloned()
    }

    pub fn build_config(&self, name: &str) -> Result<Box<dyn config::Config>> {
        match self.registry.get(name) {
            Some(metadata) => Ok(metadata.build_config()),
            None => Err(format!("no config constructor for {name} found"))?,
        }
    }
}
