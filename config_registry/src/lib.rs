// Config Registry API is a mess
// Config registry is used everywhere:
// - UI use it to render and validate forms
// - Control plane use it for input validation/secret stripping
// - Myceliald use it for input validation and section start
//
// Since runtime of daemon is type erased and use Box<dyn Section> pretty much everywhere - we need to have
// a subtrait for configs, since structs which implement configuration are also sections.
// It's not hard merge Config and Section together, but section interface, at least in current runtime implementation is
// generic over SectionChannel.
// This fact makes implementation of config regsisty tricky, since 'config only' version for ui and control plane don't need to know
// anything about section channel.
// Generic parameter turns generic everything it touches.
// It would be nice to just have DynSection with dyn SectionChannel implementation, but it will take some time to do.
// So instead we have this giant mess of a library.
#[cfg_attr(not(feature = "section"), path = "config_only.rs")]
#[cfg_attr(feature = "section", path = "config_section.rs")]
mod config_registry_impl;

pub use config_registry_impl::{Config, ConfigMetaData, ConfigRegistry};
use section::prelude::SectionChannel;

pub(crate) type Result<T, E = Box<dyn std::error::Error + Send + Sync + 'static>> =
    std::result::Result<T, E>;

pub fn new<Chan: SectionChannel>() -> Result<ConfigRegistry<Chan>> {
    let mut registry = ConfigRegistry::new();
    registry.add_config(|| Box::from(csv_transform::FromCsv::default()))?;
    registry.add_config(|| Box::from(csv_transform::ToCsv::default()))?;
    registry.add_config(|| Box::from(dir::DirSource::default()))?;
    registry.add_config(|| Box::from(excel_connector::Excel::default()))?;
    Ok(registry)
}
