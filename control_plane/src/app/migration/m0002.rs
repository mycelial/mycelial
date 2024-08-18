use sea_query::{ColumnDef, ForeignKey, ForeignKeyAction, Iden, Index, SchemaBuilder, Table};
use sqlx::migrate::{Migration, MigrationType};
use super::m0001;

// Graph nodes

#[derive(Iden)]
enum Nodes {
    Table,
    Id,
    DisplayName,
    UserId,
    WorkspaceId,
    DaemonId,
    Config,
    X,
    Y,
}

// FIXME: indices
impl Nodes {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Table::create()
            .table(Nodes::Table)
            .col(ColumnDef::new(Nodes::Id).uuid().primary_key().not_null())
            .col(ColumnDef::new(Nodes::DisplayName).string())
            .col(ColumnDef::new(Nodes::X).double().not_null())
            .col(ColumnDef::new(Nodes::Y).double().not_null())
            .col(ColumnDef::new(Nodes::UserId).big_integer())
            .col(ColumnDef::new(Nodes::WorkspaceId).big_integer().not_null())
            .col(ColumnDef::new(Nodes::DaemonId).big_integer())
            .col(ColumnDef::new(Nodes::Config).json().not_null())
            .foreign_key(
                ForeignKey::create()
                    .from(Nodes::Table, Nodes::WorkspaceId)
                    .to(m0001::Workspaces::Table, m0001::Workspaces::Id)
                    .on_update(ForeignKeyAction::Cascade)
                    .on_delete(ForeignKeyAction::Cascade)
            )
            .build_any(schema_builder)
    }
}

#[derive(Iden)]
enum Edges {
    Table,
    FromId,
    ToId,
}

impl Edges {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Table::create()
            .table(Edges::Table)
            .col(
                ColumnDef::new(Edges::FromId)
                    .uuid()
                    .primary_key()
                    .not_null(),
            )
            .col(ColumnDef::new(Edges::ToId).uuid().not_null())
            .foreign_key(
                ForeignKey::create()
                    .from(Edges::Table, Edges::FromId)
                    .to(Nodes::Table, Nodes::Id)
                    .on_update(ForeignKeyAction::Cascade)
                    .on_delete(ForeignKeyAction::Cascade)
            )
            .foreign_key(
                ForeignKey::create()
                    .from(Edges::Table, Edges::ToId)
                    .to(Nodes::Table, Nodes::Id)
                    .on_update(ForeignKeyAction::Cascade)
                    .on_delete(ForeignKeyAction::Cascade)
            )
            .build_any(schema_builder)
    }
}
struct EdgesToIdIndex {}

impl EdgesToIdIndex {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Index::create()
            .name("edges_to_id_idx")
            .table(Edges::Table)
            .col(Edges::ToId)
            .build_any(schema_builder)
    }
}

#[derive(Iden)]
enum Certs {
    Table,
    Key,
    Value,
    CreatedAt,
}

impl Certs {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Table::create()
            .table(Certs::Table)
            .col(ColumnDef::new(Certs::Key).string().primary_key().not_null())
            .col(ColumnDef::new(Certs::Value).string().not_null())
            .col(ColumnDef::new(Certs::CreatedAt).timestamp().not_null())
            .build_any(schema_builder)
    }
}

#[derive(Iden)]
enum Daemons {
    Table,
    Id,
    DisplayName,
    Address,
    LastOnline,
    JoinedAt,
}

impl Daemons {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Table::create()
            .table(Daemons::Table)
            .col(
                ColumnDef::new(Daemons::Id)
                    .big_integer()
                    .primary_key()
                    .not_null(),
            )
            .col(ColumnDef::new(Daemons::DisplayName).string().null())
            .col(ColumnDef::new(Daemons::Address).string().null())
            .col(ColumnDef::new(Daemons::LastOnline).timestamp().null())
            .col(ColumnDef::new(Daemons::JoinedAt).timestamp())
            .build_any(schema_builder)
    }
}

#[derive(Iden)]
enum DaemonTokens {
    Table,
    Id,
    Secret,
    IssuedAt,
    UsedAt,
}

impl DaemonTokens {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Table::create()
            .table(DaemonTokens::Table)
            .col(
                ColumnDef::new(DaemonTokens::Id)
                    .string()
                    .primary_key()
                    .not_null(),
            )
            .col(ColumnDef::new(DaemonTokens::Secret).string().not_null())
            .col(
                ColumnDef::new(DaemonTokens::IssuedAt)
                    .timestamp()
                    .not_null(),
            )
            .col(ColumnDef::new(DaemonTokens::UsedAt).timestamp())
            .build_any(schema_builder)
    }
}

pub fn into_migration(schema_builder: &dyn SchemaBuilder) -> Migration {
    let sql = [
        // tables
        Nodes::into_query(schema_builder),
        Edges::into_query(schema_builder),
        Certs::into_query(schema_builder),
        Daemons::into_query(schema_builder),
        DaemonTokens::into_query(schema_builder),
        // indices
        EdgesToIdIndex::into_query(schema_builder)
    ]
    .join(";\n");
    Migration::new(
        2,
        "nodes_and_edges".into(),
        MigrationType::Simple,
        sql.into(),
    )
}
