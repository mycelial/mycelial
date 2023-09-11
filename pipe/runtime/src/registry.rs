//! Registry
//!
//! Registry is a mapping between section name and section constuctor.
//! Registry is used mainly by pipe scheduler to build pipelines out of text configs.

use crate::{
    config::Map,
    types::{DynSection, SectionError},
};
use std::collections::HashMap;

pub type Constructor = fn(&Map) -> Result<Box<dyn DynSection>, SectionError>;

pub struct Registry {
    reg: HashMap<String, Constructor>,
}

impl std::fmt::Debug for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Registry {{}}")
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    pub fn new() -> Self {
        Self {
            reg: HashMap::new(),
        }
    }

    pub fn register_section(&mut self, name: impl Into<String>, init: Constructor) {
        self.reg.insert(name.into(), init);
    }

    pub fn unregister_section(&mut self, name: impl AsRef<str>) {
        self.reg.remove(name.as_ref());
    }

    pub fn get_constructor(&self, name: impl AsRef<str>) -> Option<Constructor> {
        self.reg.get(name.as_ref()).copied()
    }
}
