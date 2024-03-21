use sea_query::{ColumnDef, Expr, ForeignKey, Iden, Index, SchemaBuilder, SchemaStatementBuilder, Table};
use sqlx::{
    migrate::{Migration, MigrationType},
    Column,
};

#[derive(Iden)]
enum Clients {
    Table,
    Id,
    DisplayName,
    UserId,
    Sources,
    Destinations,
    UniqueClientId,
    ClientSecretHash,
}

impl Clients {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Table::create()
            .table(Clients::Table)
            .col(ColumnDef::new(Clients::Id).string().primary_key())
            .col(ColumnDef::new(Clients::DisplayName).string())
            .col(ColumnDef::new(Clients::UserId).string())
            .col(ColumnDef::new(Clients::Sources).json())
            .col(ColumnDef::new(Clients::Destinations).json())
            .col(ColumnDef::new(Clients::UniqueClientId).string())
            .col(ColumnDef::new(Clients::ClientSecretHash).string())
            .build_any(schema_builder)
    }
}

struct ClientsUserIdIndex;
impl ClientsUserIdIndex {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Index::create()
            .name("clients_user_id_idx")
            .table(Clients::Table)
            .col(Clients::UserId)
            .build_any(schema_builder)
    }
}

#[derive(Iden)]
enum Messages {
    Table,
    Id,
    Topic,
    Origin,
    StreamType,
    CreatedAt,
}

impl Messages {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Table::create()
            .table(Messages::Table)
            .col(
                ColumnDef::new(Messages::Id)
                    .big_integer()
                    .primary_key()
                    .auto_increment(),
            )
            .col(ColumnDef::new(Messages::Topic).string())
            .col(ColumnDef::new(Messages::Origin).string())
            .col(ColumnDef::new(Messages::StreamType).string())
            .col(
                ColumnDef::new(Messages::CreatedAt)
                    .timestamp_with_time_zone()
                    .default(Expr::current_date()),
            )
            .build_any(schema_builder)
    }
}

struct MessagesTopicIndex;

impl MessagesTopicIndex {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Index::create()
            .name("messages_topic_idx")
            .table(Messages::Table)
            .col(Messages::Topic)
            .build_any(schema_builder)
    }
}


#[derive(Iden)]
enum MessageChunks {
    Table,
    MessageId,
    ChunkId,
    Data,
}

impl MessageChunks {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Table::create()
            .table(MessageChunks::Table)
            .col(ColumnDef::new(MessageChunks::MessageId).big_integer())
            .col(ColumnDef::new(MessageChunks::ChunkId).integer())
            .col(ColumnDef::new(MessageChunks::Data).binary())
            .build_any(schema_builder)
    }
}

struct MessageChunksMessageIdIndex;
impl MessageChunksMessageIdIndex{
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Index::create()
            .name("message_chunks_message_id_idx")
            .table(MessageChunks::Table)
            .col(MessageChunks::MessageId)
            .build_any(schema_builder)
    }
}

struct MessageChunksChunkIdIndex;
impl MessageChunksChunkIdIndex {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Index::create()
            .name("message_chunks_chunk_id_idx")
            .table(MessageChunks::Table)
            .col(MessageChunks::ChunkId)
            .build_any(schema_builder)
    }
}

#[derive(Iden)]
enum Workspaces {
    Table,
    Id,
    UserId,
    Name,
    CreatedAt,
}

impl Workspaces {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Table::create()
            .table(Workspaces::Table)
            .col(
                ColumnDef::new(Workspaces::Id)
                    .integer()
                    .primary_key()
                    .auto_increment(),
            )
            .col(ColumnDef::new(Workspaces::UserId).string())
            .col(ColumnDef::new(Workspaces::Name).string())
            .col(
                ColumnDef::new(Workspaces::CreatedAt)
                    .timestamp_with_time_zone()
                    .default(Expr::current_timestamp()),
            )
            .build_any(schema_builder)
    }
}

#[derive(Iden)]
enum Pipes {
    Table,
    Id,
    UserId,
    WorkspaceId,
    RawConfig,
    CreatedAt,
}

impl Pipes {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Table::create()
            .table(Pipes::Table)
            .col(
                ColumnDef::new(Pipes::Id)
                    .big_integer()
                    .primary_key()
                    .auto_increment(),
            )
            .col(ColumnDef::new(Pipes::UserId).string())
            .col(ColumnDef::new(Pipes::WorkspaceId).integer())
            .col(ColumnDef::new(Pipes::RawConfig).json())
            .col(
                ColumnDef::new(Pipes::CreatedAt)
                    .timestamp_with_time_zone()
                    .default(Expr::current_timestamp()),
            )
            .build_any(schema_builder)
    }
}

struct PipesUserIdIndex;

impl PipesUserIdIndex {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Index::create()
            .name("pipes_user_id_idx")
            .table(Pipes::Table)
            .col(Pipes::UserId)
            .build_any(schema_builder)
    }
}

#[derive(Iden)]
enum UserDaemonTokens {
    Table,
    UserId,
    DaemonToken,
}

impl UserDaemonTokens {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Table::create()
            .table(UserDaemonTokens::Table)
            .col(
                ColumnDef::new(UserDaemonTokens::UserId)
                    .string()
                    .primary_key(),
            )
            .col(ColumnDef::new(UserDaemonTokens::DaemonToken).string())
            .build_any(schema_builder)
    }
}

struct UserDaemonTokensIndexDaemonToken;
impl UserDaemonTokensIndexDaemonToken {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Index::create()
            .name("user_daemon_tokens_daemon_token_idx")
            .table(Messages::Table)
            .col(Messages::Topic)
            .build_any(schema_builder)
    }
}

pub fn into_migration(schema_builder: &dyn SchemaBuilder) -> Migration {
    let sql = [
        // tables
        Clients::into_query(schema_builder),
        Messages::into_query(schema_builder),
        MessageChunks::into_query(schema_builder),
        Workspaces::into_query(schema_builder),
        Pipes::into_query(schema_builder),
        UserDaemonTokens::into_query(schema_builder),

        // indices
        ClientsUserIdIndex::into_query(schema_builder),
        MessagesTopicIndex::into_query(schema_builder),
        MessageChunksMessageIdIndex::into_query(schema_builder),
        MessageChunksChunkIdIndex::into_query(schema_builder),
        PipesUserIdIndex::into_query(schema_builder),
        UserDaemonTokensIndexDaemonToken::into_query(schema_builder),

    ]
    .join(";\n");
    Migration::new(1, "initial".into(), MigrationType::Simple, sql.into())
}
