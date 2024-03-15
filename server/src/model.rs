use std::pin::Pin;
use crate::Result;
use chrono::{DateTime, Utc};
use common::{Destination, PipeConfig, Source};
use futures::Stream;
use serde::{Deserialize, Serialize};

pub struct MessageStream {
    pub id: u64,
    pub origin: String,
    pub stream_type: String,
    pub stream: Pin<Box<dyn Stream<Item = Result<Vec<u8>>> + Send>>,
}

// FIXME: rename clients do daemons
#[derive(Serialize, Deserialize, Debug)]
pub struct Clients {
    pub clients: Vec<Client>,
}

// FIXME: rename clients to daemons
#[derive(Serialize, Deserialize, Debug)]
pub struct Client {
    pub id: String,
    pub display_name: String,
    pub sources: Vec<Source>,
    pub destinations: Vec<Destination>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Workspace {
    #[serde(default)]
    pub id: i32,
    pub name: String,
    #[serde(default)]
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub pipe_configs: Vec<PipeConfig>,
}
