mod control_plane_client;
mod runtime;
mod runtime_error;
mod runtime_storage;
mod scheduler;
mod section_channel;
mod sqlite_storage;

use config_registry::{Config as _Config, ConfigRegistry as _ConfigRegistry};
use uuid::Uuid;

pub(crate) type SectionChannel = section_channel::SectionChannel<Uuid, sqlite_storage::SqliteState>;
pub(crate) type ConfigRegistry = _ConfigRegistry<SectionChannel>;
pub(crate) type Config = Box<dyn _Config<SectionChannel>>;

pub(crate) type Result<T, E = runtime_error::RuntimeError> = std::result::Result<T, E>;

pub async fn new(database_path: &str) -> Result<runtime::Runtime> {
    runtime::Runtime::new(database_path).await
}