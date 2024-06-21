use std::collections::BTreeMap;

use config::prelude::*;
use crate::Result;

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
    registry: BTreeMap<String, ConfigConstructor>
}

impl ConfigRegistry {
    pub fn new() -> Self {
        let mut s = Self {
            registry: BTreeMap::new()
        };
        s.add_config(|| Box::from(ConfigExample::default())).unwrap();
        s.add_config(|| Box::from(ConfigExampleSecond::default())).unwrap();
        s
    }
    
    pub fn add_config(&mut self, ctor: ConfigConstructor) -> Result<()> {
        let config = ctor();
        let name = config.name();
        if let Some(_) = self.registry.get(name) {
            Err(format!("{name} already present"))?
        };
        self.registry.insert(name.into(), ctor);
        Ok(())
    }
    
    pub fn keys(&self) -> impl Iterator<Item=&str> {
        self.registry.keys().map(|key| key.as_str())
    }
    
    pub fn build_config(&self, name: &str) -> Result<Box<dyn config::Config>> {
        match self.registry.get(name) {
            Some(ctor) => Ok(ctor()),
            None => Err(format!("no config constructor for {name} found"))?
        }
    }
}