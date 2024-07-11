use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    ops::{Deref, DerefMut},
};

use crate::app::migration;
use crate::{
    app::tables::{Edges, Nodes, Workspaces},
    app::{AppError, Result},
};
use chrono::{DateTime, NaiveDateTime, Utc};
use derive_trait::derive_trait;
use futures::future::BoxFuture;
use sea_query::{
    Expr, Order, PostgresQueryBuilder, Query, QueryBuilder, SchemaBuilder, SqliteQueryBuilder,
};
use sea_query_binder::{SqlxBinder, SqlxValues};
use sqlx::{
    database::HasArguments, migrate::Migrate, types::Json, ColumnIndex, Connection as _, Database,
    Executor, IntoArguments, Pool, Postgres, Row as _, Sqlite, Transaction,
};
use uuid::Uuid;

use super::{Edge, Graph, Node, Workspace, WorkspaceOperation, WorkspaceUpdate};

// FIXME: pool options and configurable pool size
pub async fn new(database_url: &str) -> Result<Box<dyn DbTrait>> {
    let mut database_url = url::Url::parse(database_url)?;
    let mut params: HashMap<Cow<str>, Cow<str>> = database_url.query_pairs().collect();
    let db: Box<dyn DbTrait> = match database_url.scheme() {
        "sqlite" => {
            // FIXME: move to util or smth?
            // without "mode=rwc" sqlite will bail if database file is not present
            match params.get("mode") {
                Some(_) => (),
                None => {
                    params.insert("mode".into(), "rwc".into());
                }
            };
            let query = params
                .into_iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join("&");
            database_url.set_query(Some(&query));
            Box::new(
                Db::<Sqlite>::new(
                    database_url.as_ref(),
                    Box::new(SqliteQueryBuilder),
                    Box::new(SqliteQueryBuilder),
                )
                .await?,
            )
        }
        "postgres" => Box::new(
            Db::<Postgres>::new(
                database_url.as_ref(),
                Box::new(PostgresQueryBuilder),
                Box::new(PostgresQueryBuilder),
            )
            .await?,
        ),
        unsupported => Err(anyhow::anyhow!("unsupported database: {unsupported}"))?,
    };
    Ok(db)
}
pub struct Db<D: Database> {
    pool: Pool<D>,
    query_builder: Box<dyn QueryBuilder + Send + Sync>,
    schema_builder: Box<dyn SchemaBuilder + Send + Sync>,
}

impl<D: Database> Db<D> {
    async fn new(
        url: &str,
        query_builder: Box<dyn QueryBuilder + Send + Sync>,
        schema_builder: Box<dyn SchemaBuilder + Send + Sync>,
    ) -> Result<Self> {
        Ok(Self {
            pool: Pool::<D>::connect(url).await?,
            query_builder,
            schema_builder,
        })
    }
}

// automatically derives new trait with Send + Sync bounds
// trait funcs are copied from impl block
#[derive_trait(Send + Sync)]
impl<D> DbTrait for Db<D>
where
    D: Database,
    // Types, that Database should support
    for<'e> i16: sqlx::Type<D> + sqlx::Encode<'e, D> + sqlx::Decode<'e, D>,
    for<'e> i32: sqlx::Type<D> + sqlx::Encode<'e, D> + sqlx::Decode<'e, D>,
    for<'e> i64: sqlx::Type<D> + sqlx::Encode<'e, D> + sqlx::Decode<'e, D>,
    for<'e> f32: sqlx::Type<D> + sqlx::Encode<'e, D> + sqlx::Decode<'e, D>,
    for<'e> f64: sqlx::Type<D> + sqlx::Encode<'e, D> + sqlx::Decode<'e, D>,
    for<'e> String: sqlx::Type<D> + sqlx::Encode<'e, D> + sqlx::Decode<'e, D>,
    for<'e> &'e str: sqlx::Type<D> + sqlx::Encode<'e, D> + sqlx::Decode<'e, D>,
    for<'e> Vec<u8>: sqlx::Type<D> + sqlx::Encode<'e, D> + sqlx::Decode<'e, D>,
    for<'e> Uuid: sqlx::Type<D> + sqlx::Encode<'e, D> + sqlx::Decode<'e, D>,
    for<'e> Json<serde_json::Value>: sqlx::Type<D> + sqlx::Encode<'e, D> + sqlx::Decode<'e, D>,
    for<'e> DateTime<Utc>: sqlx::Type<D> + sqlx::Encode<'e, D> + sqlx::Decode<'e, D>,
    for<'e> NaiveDateTime: sqlx::Type<D> + sqlx::Encode<'e, D> + sqlx::Decode<'e, D>,
    for<'e> serde_json::Value: sqlx::Type<D> + sqlx::Encode<'e, D> + sqlx::Decode<'e, D>,

    // col access through usize index
    usize: ColumnIndex<D::Row>,

    // sea-query-binder
    for<'e> SqlxValues: IntoArguments<'e, D>,

    // sqlx bounds
    for<'c> &'c mut <D as Database>::Connection: Executor<'c, Database = D>,
    for<'e> &'e Pool<D>: Executor<'e, Database = D>,
    for<'q> <D as HasArguments<'q>>::Arguments: IntoArguments<'q, D>,
    D::QueryResult: std::fmt::Debug,

    // Database transactions should be deref-able into database connection
    for<'e> Transaction<'e, D>: Deref<Target = <D as Database>::Connection>,
    for<'e> Transaction<'e, D>: DerefMut<Target = <D as Database>::Connection>,

    // db connection should be able to run migrations
    D::Connection: Migrate,
{
    // migrate database to latest state
    fn migrate(&self) -> BoxFuture<Result<()>> {
        Box::pin(async {
            migration::migrate(&self.pool, &*self.schema_builder).await?;
            Ok(())
        })
    }

    // workspaces API
    fn create_workspace<'a>(&'a self, workspace: &'a Workspace) -> BoxFuture<'a, Result<()>> {
        Box::pin(async {
            let created_at = chrono::Utc::now();
            let (query, values) = Query::insert()
                .columns([
                    // FIXME: should be unique for (name, user_id) pair
                    Workspaces::Name,
                    // FIXME: it's empty now
                    Workspaces::UserId,
                    Workspaces::CreatedAt,
                ])
                .into_table(Workspaces::Table)
                .values_panic([
                    workspace.name.as_str().into(),
                    // FIXME: user_id is empty
                    "".into(),
                    created_at.into(),
                ])
                .build_any_sqlx(&*self.query_builder);
            sqlx::query_with(&query, values).execute(&self.pool).await?;
            Ok(())
        })
    }

    fn read_workspaces(&self) -> BoxFuture<'_, Result<Vec<Workspace>>> {
        Box::pin(async {
            let (query, values) = Query::select()
                .columns([Workspaces::Name, Workspaces::CreatedAt])
                .from(Workspaces::Table)
                .and_where(Expr::col(Workspaces::UserId).eq(""))
                .order_by(Workspaces::CreatedAt, Order::Asc)
                .build_any_sqlx(&*self.query_builder);
            let workspaces = sqlx::query_with(&query, values)
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .map(|row| Workspace {
                    name: row.get(0),
                    created_at: Some(row.get(1)),
                })
                .collect::<Vec<Workspace>>();
            Ok(workspaces)
        })
    }

    fn delete_workspace<'a>(&'a self, name: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let (query, values) = Query::delete()
                .from_table(Workspaces::Table)
                .and_where(Expr::col(Workspaces::Name).eq(name))
                .build_any_sqlx(&*self.query_builder);
            sqlx::query_with(&query, values).execute(&self.pool).await?;
            Ok(())
        })
    }

    // Workspace api
    fn get_graph<'a>(&'a self, workspace_name: &'a str) -> BoxFuture<'a, Result<Graph>> {
        Box::pin(async move {
            let (query, values) = Query::select()
                .columns([Workspaces::Id])
                .from(Workspaces::Table)
                .and_where(Expr::col(Workspaces::Name).eq(workspace_name))
                .build_any_sqlx(&*self.query_builder);
            let workspace_id: i64 = match sqlx::query_with(&query, values)
                .fetch_optional(&self.pool)
                .await?
            {
                Some(row) => row.get(0),
                None => Err(AppError::not_found(anyhow::anyhow!(
                    "workspace '{workspace_name}"
                )))?,
            };

            let (query, values) = Query::select()
                .columns([
                    Nodes::Id,
                    Nodes::DisplayName,
                    Nodes::Config,
                    Nodes::X,
                    Nodes::Y,
                ])
                .from(Nodes::Table)
                .and_where(Expr::col(Nodes::WorkspaceId).eq(workspace_id))
                .build_any_sqlx(&*self.query_builder);
            let nodes = sqlx::query_with(&query, values)
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .map(|row| {
                    let config = row.get::<Json<_>, _>(2).0;
                    Ok(Node {
                        id: row.get(0),
                        display_name: row.get(1),
                        config: serde_json::from_value(config)?,
                        x: row.get(3),
                        y: row.get(4),
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            let node_ids = nodes.iter().map(|node| node.id);
            let (query, values) = Query::select()
                .columns([Edges::FromId, Edges::ToId])
                .from(Edges::Table)
                .and_where(Expr::col(Edges::FromId).is_in(node_ids))
                .build_any_sqlx(&*self.query_builder);
            let edges = sqlx::query_with(&query, values)
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .map(|row| Edge {
                    from_id: row.get(0),
                    to_id: row.get(1),
                })
                .collect::<Vec<_>>();
            Ok(Graph { nodes, edges })
        })
    }

    fn update_workspace<'a>(&'a self, updates: &'a [WorkspaceUpdate]) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let conn = &mut *self.pool.acquire().await?;
            let mut transaction = conn.begin().await?;
            let mut workspaces_cache = BTreeMap::<String, i64>::new();
            for update in updates {
                let workspace_name = update.name.as_str();
                let workspace_id = match workspaces_cache.get(workspace_name) {
                    None => {
                        let (query, values) = Query::select()
                            .columns([Workspaces::Id])
                            .from(Workspaces::Table)
                            .and_where(Expr::col(Workspaces::Name).eq(workspace_name))
                            .build_any_sqlx(&*self.query_builder);
                        let id = match sqlx::query_with(&query, values)
                            .fetch_optional(&mut *transaction)
                            .await?
                        {
                            Some(row) => row.get::<i64, _>(0),
                            None => Err(anyhow::anyhow!("failed to perform workspace update: no such workspace {workspace_name}"))?,  
                        };
                        workspaces_cache.insert(workspace_name.to_string(), id);
                        id
                    }
                    Some(id) => *id,
                };
                for op in update.operations.iter() {
                    let (query, values) = match *op {
                        WorkspaceOperation::AddNode {
                            id,
                            x,
                            y,
                            ref config,
                        } => {
                            let config_json = serde_json::to_string(config)?;
                            Query::insert()
                                .columns([
                                    Nodes::Id,
                                    Nodes::X,
                                    Nodes::Y,
                                    Nodes::Config,
                                    Nodes::WorkspaceId,
                                ])
                                .into_table(Nodes::Table)
                                .values_panic([
                                    id.into(),
                                    x.into(),
                                    y.into(),
                                    config_json.into(),
                                    workspace_id.into(),
                                ])
                                .build_any_sqlx(&*self.query_builder)
                        }
                        WorkspaceOperation::RemoveNode(uuid) => Query::delete()
                            .from_table(Nodes::Table)
                            .and_where(Expr::col(Nodes::Id).eq(uuid))
                            .build_any_sqlx(&*self.query_builder),
                        WorkspaceOperation::UpdateNodePosition { uuid, x, y } => Query::update()
                            .table(Nodes::Table)
                            .values([(Nodes::X, x.into()), (Nodes::Y, y.into())])
                            .and_where(Expr::col(Nodes::Id).eq(uuid))
                            .build_any_sqlx(&*self.query_builder),
                        WorkspaceOperation::AddEdge { from, to } => Query::insert()
                            .columns([Edges::FromId, Edges::ToId])
                            .into_table(Edges::Table)
                            .values_panic([from.into(), to.into()])
                            .build_any_sqlx(&*self.query_builder),
                        WorkspaceOperation::RemoveEdge { from } => Query::delete()
                            .from_table(Edges::Table)
                            .and_where(Expr::col(Edges::FromId).eq(from))
                            .build_any_sqlx(&*self.query_builder),
                    };
                    sqlx::query_with(&query, values)
                        .execute(&mut *transaction)
                        .await?;
                }
            }
            transaction.commit().await?;
            Ok(())
        })
    }
}
