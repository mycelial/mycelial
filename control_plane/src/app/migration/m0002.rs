use sea_query::{ColumnDef, Iden, SchemaBuilder, Table};
use sqlx::migrate::{Migration, MigrationType};

// Graph nodes

#[derive(Iden)]
enum Nodes {
    Table,
    Id,
    DisplayName,
    NodeType,
    UserId,
    WorkspaceId,
    DaemonId,
}

// FIXME: indices
impl Nodes {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Table::create()
            .table(Nodes::Table)
            .col(ColumnDef::new(Nodes::Id).uuid().primary_key())
            .col(ColumnDef::new(Nodes::DisplayName).string())
            .col(ColumnDef::new(Nodes::NodeType).string())
            .col(ColumnDef::new(Nodes::UserId).big_integer())
            .col(ColumnDef::new(Nodes::WorkspaceId).big_integer())
            .col(ColumnDef::new(Nodes::DaemonId).big_integer())
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
            .col(ColumnDef::new(Edges::FromId).uuid().primary_key())
            .col(ColumnDef::new(Edges::ToId).uuid())
            .build_any(schema_builder)
    }
}

#[derive(Iden)]
enum NodeParam {
    Table,
    NodeId,
    Key,
    // FIXME: how to store nested data?
    Value,
}

// FIXME: index on node id
impl NodeParam {
    fn into_query(schema_builder: &dyn SchemaBuilder) -> String {
        Table::create()
            .table(NodeParam::Table)
            .col(ColumnDef::new(NodeParam::NodeId).uuid())
            .col(ColumnDef::new(NodeParam::Key).string())
            .col(ColumnDef::new(NodeParam::Value).string())
            .build_any(schema_builder)
    }
}

pub fn into_migration(schema_builder: &dyn SchemaBuilder) -> Migration {
    let sql = [
        // tables
        Nodes::into_query(schema_builder),
        Edges::into_query(schema_builder),
        NodeParam::into_query(schema_builder),
        // indices
    ]
    .join(";\n");
    Migration::new(
        2,
        "nodes_and_edges".into(),
        MigrationType::Simple,
        sql.into(),
    )
}
