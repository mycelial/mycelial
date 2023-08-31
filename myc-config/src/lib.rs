use serde::{Deserialize, Serialize};

/// Top level configuration object
// todo: should this be only on the client?
#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub node: Node,
    pub server: Server,
    pub sources: Vec<Source>,
}

/// Client-side server config
#[derive(Serialize, Deserialize, Debug)]
pub struct Server {
    pub endpoint: String,
    pub token: String,
}

/// Generic node config
#[derive(Serialize, Deserialize, Debug)]
pub struct Node {
    pub display_name: String,
    pub unique_id: String,
    pub storage_path: String,
}

/// Internally-tagged type of a source needs to match the variant name
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Source {
    Sqlite(SqliteConfig),
    Kafka(KafkaConfig),
    Postgres(PostgresConfig),
    Snowflake(SnowflakeConfig),
}

// Shared between all source definitions
#[derive(Serialize, Deserialize, Debug)]
pub struct CommonAttrs {
    // pub r#type: String,
    pub display_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SqliteConfig {
    #[serde(flatten)]
    pub common_attrs: CommonAttrs,
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KafkaConfig {
    #[serde(flatten)]
    pub common_attrs: CommonAttrs,
    // comma-separated
    pub brokers: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PostgresConfig {
    #[serde(flatten)]
    pub common_attrs: CommonAttrs,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SnowflakeConfig {
    #[serde(flatten)]
    pub common_attrs: CommonAttrs,
    pub username: String,
    pub password: String,
    pub role: String,
    pub account_identifier: String,
    pub warehouse: String,
    pub database: String,
}