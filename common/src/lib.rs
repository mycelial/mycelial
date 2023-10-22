use pipe::{
    config::{Config as DynamicPipeConfig, Value as DynamicPipeValue},
    types::SectionError,
};
use serde::{Deserialize, Serialize};

/// Top level configuration object
// todo: should this be only on the client?
// todo: disallow clone
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClientConfig {
    pub node: Node,
    pub server: Server,
    #[serde(default)]
    pub sources: Vec<Source>,
    #[serde(default)]
    pub destinations: Vec<Destination>,
}

/// Client-side server config
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Server {
    pub endpoint: String,
    pub token: String,
}

/// Generic node config
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Node {
    pub display_name: String,
    pub unique_id: String,
    pub storage_path: String,
}

/// Internally-tagged type of a source needs to match the variant name
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Source {
    Sqlite_Connector(SqliteConnectorConfig),
    Kafka(KafkaConfig),
    Postgres(PostgresConfig),
    Snowflake(SnowflakeConfig),
    Sqlite_Physical_Replication(SqlitePhysicalReplicationSourceConfig),
    Hello_World(HelloWorldConfig),
    Bacalhau(BacalhauConfig),
}

/// Internally-tagged type of a source needs to match the variant name
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Destination {
    Sqlite_Connector(SqliteConnectorConfig),
    Snowflake(SnowflakeConfig),
    Sqlite_Physical_Replication(SqlitePhysicalReplicationDestinationConfig),
    Hello_World(HelloWorldConfig),
    Kafka(KafkaDestinationConfig),
    Bacalhau(BacalhauConfig),
}

// Shared between all source definitions
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommonAttrs {
    // pub r#type: String,
    pub display_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SqliteConnectorConfig {
    #[serde(flatten)]
    pub common_attrs: CommonAttrs,
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KafkaConfig {
    #[serde(flatten)]
    pub common_attrs: CommonAttrs,
    // comma-separated
    pub brokers: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PostgresConfig {
    #[serde(flatten)]
    pub common_attrs: CommonAttrs,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SqlitePhysicalReplicationSourceConfig {
    #[serde(flatten)]
    pub common_attrs: CommonAttrs,
    pub journal_path: String,
    // database path
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HelloWorldConfig {
    #[serde(flatten)]
    pub common_attrs: CommonAttrs,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BacalhauConfig {
    #[serde(flatten)]
    pub common_attrs: CommonAttrs,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KafkaDestinationConfig {
    #[serde(flatten)]
    pub common_attrs: CommonAttrs,
    pub brokers: String,
    pub topic: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SqlitePhysicalReplicationDestinationConfig {
    #[serde(flatten)]
    pub common_attrs: CommonAttrs,
    pub journal_path: String,
    pub database_path: String,
}

// requests and responses
// todo: move to a module

#[derive(Serialize, Deserialize, Debug)]
pub struct ProvisionClientRequest {
    pub client_config: ClientConfig,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProvisionClientResponse {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IssueTokenRequest {
    pub client_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IssueTokenResponse {
    pub id: String,
    pub client_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PipeConfigs {
    pub configs: Vec<PipeConfig>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PipeConfig {
    /// Scheduler needs to maintain pipe processes:
    /// - start new pipes
    /// - restart pipes on configuration update
    /// - stop pipes, when pipe was removed from configuration list
    ///
    /// To do that - each pipe config needs to be uniquely identified, so here is i64 integer to
    /// help with that. Signed due to Sqlite backed storage
    pub id: u64,

    /// # Example of config
    /// ```json
    /// {"configs": [
    ///     {
    ///         "id":1,
    ///         "pipe": [
    ///             {
    ///                 "name": "sqlite",
    ///                 "path": "/tmp/test.sqlite",
    ///                 "query": "select * from test"
    ///             },
    ///             {
    ///                 "endpoint": "http://localhost:8080/ingestion",
    ///                 "name": "mycelial_server",
    ///                 "token": "mycelial_server_token"
    ///             }
    ///         ]
    /// }]}
    /// ```
    pub pipe: serde_json::Value,
}

impl TryInto<DynamicPipeConfig> for PipeConfig {
    type Error = SectionError;

    fn try_into(self) -> Result<DynamicPipeConfig, Self::Error> {
        let value: DynamicPipeValue = self.pipe.try_into()?;
        DynamicPipeConfig::try_from(value)
    }
}
