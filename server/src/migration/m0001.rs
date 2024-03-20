use sea_query::{ColumnDef, Expr, ForeignKey, Iden, SchemaBuilder, SchemaStatementBuilder, Table};
use sqlx::{migrate::{Migration, MigrationType}, Column};

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

#[derive(Iden)]
enum Tokens {
    Table,
    Id,
    ClientId,
    CreatedAt,
}

impl Tokens {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Table::create()
            .table(Tokens::Table)
            .col(ColumnDef::new(Tokens::Id).string().primary_key())
            .col(ColumnDef::new(Tokens::ClientId).string().not_null())
            .col(ColumnDef::new(Tokens::CreatedAt).timestamp_with_time_zone())
            .foreign_key(
                ForeignKey::create()
                    .name("tokens_client_id_ref")
                    .from(Tokens::Table, Tokens::ClientId)
                    .to(Clients::Table, Clients::Id)
            )
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
}

impl Messages {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Table::create()
            .table(Messages::Table)
            .col(ColumnDef::new(Messages::Id).big_integer().primary_key().auto_increment())
            .col(ColumnDef::new(Messages::Topic).string())
            .col(ColumnDef::new(Messages::Origin).string())
            .col(ColumnDef::new(Messages::StreamType).string())
            .build_any(schema_builder)
    }
}

#[derive(Iden)]
enum Records {
    Table,
    Id,
    MessageId,
    CreatedAt,
    Data,
}

impl Records {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Table::create()
            .table(Records::Table)
            .col(ColumnDef::new(Records::Id).big_integer().primary_key().auto_increment())
            .col(ColumnDef::new(Records::MessageId).big_integer())
            .col(ColumnDef::new(Records::CreatedAt).timestamp_with_time_zone())
            .col(ColumnDef::new(Records::Data).binary())
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
            .col(ColumnDef::new(Workspaces::Id).integer().primary_key().auto_increment())
            .col(ColumnDef::new(Workspaces::UserId).string())
            .col(ColumnDef::new(Workspaces::Name).string())
            .col(ColumnDef::new(Workspaces::CreatedAt).timestamp_with_time_zone().default(Expr::current_timestamp()))
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
            .col(ColumnDef::new(Pipes::Id).big_integer().primary_key().auto_increment())
            .col(ColumnDef::new(Pipes::UserId).string())
            .col(ColumnDef::new(Pipes::WorkspaceId).integer())
            .col(ColumnDef::new(Pipes::RawConfig).json())
            .col(ColumnDef::new(Pipes::CreatedAt).timestamp_with_time_zone().default(Expr::current_timestamp()))
            .build_any(schema_builder)
    }
}

#[derive(Iden)]
enum UserDaemonTokens{
    Table,
    UserId,
    DaemonToken
}

impl UserDaemonTokens {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Table::create()
            .table(UserDaemonTokens::Table)
            .col(ColumnDef::new(UserDaemonTokens::UserId).string().primary_key())
            .col(ColumnDef::new(UserDaemonTokens::DaemonToken).string())
            .build_any(schema_builder)
    }
}

pub fn into_migration(schema_builder: &dyn SchemaBuilder) -> Migration {
    let sql = [
        Clients::into_query(schema_builder),
        Tokens::into_query(schema_builder),
        Messages::into_query(schema_builder),
        Records::into_query(schema_builder),
        Workspaces::into_query(schema_builder),
        Pipes::into_query(schema_builder),
        UserDaemonTokens::into_query(schema_builder),
    ].join(";\n");
    Migration::new(1, "initial".into(), MigrationType::Simple, sql.into())
}
