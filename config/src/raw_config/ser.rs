use serde::Serialize;

use crate::Config;

use super::RawConfig;

impl Serialize for RawConfig {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let config: &dyn Config = self;
        config.serialize(serializer)
    }
}
