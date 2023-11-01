//! Registry
//!
//! Registry is a mapping between section name and section constuctor.
//! Registry is used mainly by pipe scheduler to build pipelines out of text configs.

use crate::{
    config::Map,
    types::{DynSection, SectionError},
};
use section::SectionChannel;
use std::collections::HashMap;

pub type Constructor<S> = fn(&Map) -> Result<Box<dyn DynSection<S>>, SectionError>;

pub struct Registry<S: SectionChannel> {
    reg: HashMap<String, Constructor<S>>,
}

impl<S: SectionChannel> std::fmt::Debug for Registry<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Registry {{}}")
    }
}

impl<S: SectionChannel> Registry<S> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            reg: HashMap::new(),
        }
    }

    pub fn register_section(&mut self, name: impl Into<String>, init: Constructor<S>) {
        self.reg.insert(name.into(), init);
    }

    pub fn unregister_section(&mut self, name: impl AsRef<str>) {
        self.reg.remove(name.as_ref());
    }

    pub fn get_constructor(&self, name: impl AsRef<str>) -> Option<Constructor<S>> {
        self.reg.get(name.as_ref()).copied()
    }
}
