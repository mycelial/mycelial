use std::{
    borrow::Cow,
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use crate::app::migration;
use crate::{
    app::tables::*,
    app::{AppError, Result},
};
use chrono::{DateTime, NaiveDateTime, Utc};
use config::prelude::deserialize_into_config;
use config_registry::ConfigRegistry;
use derive_trait::derive_trait;
use futures::future::BoxFuture;
use sea_query::{
    Expr, MysqlQueryBuilder, Order, PostgresQueryBuilder, Query, QueryBuilder, SchemaBuilder,
    SqliteQueryBuilder,
};
use sea_query_binder::{SqlxBinder, SqlxValues};
use sqlx::{
    database::HasArguments, migrate::Migrate, types::Json, ColumnIndex, Connection as _, Database,
    Executor, IntoArguments, MySql, Pool, Postgres, Row as _, Sqlite, Transaction,
};
use uuid::Uuid;

use super::{
    Daemon, DaemonGraph, DaemonNode, DaemonToken, Edge, Workspace, WorkspaceGraph, WorkspaceNode,
    WorkspaceOperation, WorkspaceUpdate,
};

// FIXME: pool options and configurable pool size
pub async fn new(database_url: &str) -> Result<Box<dyn DbTrait>> {
    let mut database_url = url::Url::parse(database_url)?;
    let mut params: HashMap<Cow<str>, Cow<str>> = database_url.query_pairs().collect();
    let db: Box<dyn DbTrait> = match database_url.scheme() {
        "sqlite" => {
            if !params.contains_key("mode") {
                params.insert("mode".into(), "rwc".into());
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
        "mysql" => Box::new(
            Db::<MySql>::new(
                database_url.as_ref(),
                Box::new(MysqlQueryBuilder),
                Box::new(MysqlQueryBuilder),
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
    fn get_workspace<'a>(
        &'a self,
        workspace_name: &'a str,
    ) -> BoxFuture<'a, Result<WorkspaceGraph>> {
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
                    Nodes::DaemonId,
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
                    Ok(WorkspaceNode::new(
                        row.get(0),
                        row.get(1),
                        serde_json::from_value(config)?,
                        row.get(3),
                        row.get(4),
                        row.get(5),
                    ))
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
            Ok(WorkspaceGraph { nodes, edges })
        })
    }

    // FIXME: split function
    fn update_workspace<'a>(
        &'a self,
        config_registry: &'a ConfigRegistry,
        update: &'a WorkspaceUpdate,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let conn = &mut *self.pool.acquire().await?;
            let mut transaction = conn.begin().await?;
            let workspace_name = update.name.as_str();
            let (query, values) = Query::select()
                .columns([Workspaces::Id])
                .from(Workspaces::Table)
                .and_where(Expr::col(Workspaces::Name).eq(workspace_name))
                .build_any_sqlx(&*self.query_builder);
            let workspace_id = match sqlx::query_with(&query, values)
                .fetch_optional(&mut *transaction)
                .await?
            {
                Some(row) => row.get::<i64, _>(0),
                None => Err(AppError::workspace_not_found(workspace_name))?,
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
                    WorkspaceOperation::UpdateNodeConfig { id, ref config } => {
                        let (query, values) = Query::select()
                            .columns([Nodes::Config])
                            .from(Nodes::Table)
                            .and_where(Expr::col(Nodes::Id).eq(id))
                            .lock_exclusive()
                            .build_any_sqlx(&*self.query_builder);
                        let mut cur_config = match sqlx::query_with(&query, values)
                            .fetch_optional(&mut *transaction)
                            .await?
                        {
                            Some(row) => {
                                let raw_config = row.get::<Json<_>, _>(0).0;
                                let raw_config: Box<dyn config::Config> =
                                    serde_json::from_value(raw_config)?;
                                let mut cur_config = config_registry
                                    .build_config(raw_config.name())
                                    .map_err(|e| {
                                        anyhow::anyhow!(
                                            "failed to build config '{}': {e}",
                                            raw_config.name()
                                        )
                                    })?;
                                deserialize_into_config(&*raw_config, &mut *cur_config).map_err(
                                    |e| {
                                        anyhow::anyhow!(
                                            "failed to deserialize into config: {}: {e}",
                                            raw_config.name()
                                        )
                                    },
                                )?;
                                cur_config
                            }
                            None => {
                                tracing::error!("can't update node config for node with id {id}, node not found");
                                continue;
                            }
                        };
                        deserialize_into_config(&**config, &mut *cur_config).map_err(|e| {
                            anyhow::anyhow!(
                                "failed to deserialize incoming update into config: {}: {e}",
                                cur_config.name()
                            )
                        })?;
                        let config_json = serde_json::to_string(&*cur_config)?;
                        Query::update()
                            .table(Nodes::Table)
                            .values([(Nodes::Config, config_json.into())])
                            .and_where(Expr::col(Nodes::Id).eq(id))
                            .build_any_sqlx(&*self.query_builder)
                    }
                    WorkspaceOperation::AssignNodeToDaemon { node_id, daemon_id } => {
                        Query::update()
                            .table(Nodes::Table)
                            .values([(Nodes::DaemonId, daemon_id.into())])
                            .and_where(Expr::col(Nodes::Id).eq(node_id))
                            .build_any_sqlx(&*self.query_builder)
                    }
                    WorkspaceOperation::UnassignNodeFromDaemon { node_id } => Query::update()
                        .table(Nodes::Table)
                        .values([(Nodes::DaemonId, Option::<Uuid>::None.into())])
                        .and_where(Expr::col(Nodes::Id).eq(node_id))
                        .build_any_sqlx(&*self.query_builder),
                };
                sqlx::query_with(&query, values)
                    .execute(&mut *transaction)
                    .await?;
            }
            transaction.commit().await?;
            Ok(())
        })
    }

    fn get_ca_cert_key(&self) -> BoxFuture<'_, Result<Option<(String, String)>>> {
        Box::pin(async move {
            let (query, values) = Query::select()
                .columns([Certs::Key, Certs::Value])
                .from(Certs::Table)
                .and_where(Expr::col(Certs::Key).is_in([Certs::ca_key(), Certs::ca_cert()]))
                .build_any_sqlx(&*self.query_builder);
            let rows = sqlx::query_with(&query, values)
                .fetch_all(&self.pool)
                .await?;
            let maybe_cert_key = rows.into_iter().fold((None, None), |(cert, key), row| {
                let row_key = row.get::<String, _>(0);
                let row_value = row.get::<String, _>(1);
                match &row_key {
                    row_key if row_key == Certs::ca_key() => (cert, Some(row_value)),
                    row_key if row_key == Certs::ca_cert() => (Some(row_value), key),
                    row_key => unreachable!("unexpected key {row_key}"),
                }
            });
            match maybe_cert_key {
                (Some(cert), Some(key)) => Ok(Some((cert, key))),
                (None, None) => Ok(None),
                (None, _) => Err(anyhow::anyhow!("ca cert is missing"))?,
                (_, None) => Err(anyhow::anyhow!("ca key is missing"))?,
            }
        })
    }

    fn store_ca_cert_key<'a>(&'a self, key: &'a str, cert: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let (query, values) = Query::insert()
                .columns([Certs::Key, Certs::Value, Certs::CreatedAt])
                .into_table(Certs::Table)
                .values_panic([
                    Certs::ca_key().into(),
                    key.into(),
                    Expr::current_timestamp().into(),
                ])
                .values_panic([
                    Certs::ca_cert().into(),
                    cert.into(),
                    Expr::current_timestamp().into(),
                ])
                .build_any_sqlx(&*self.query_builder);
            sqlx::query_with(&query, values).execute(&self.pool).await?;
            Ok(())
        })
    }

    fn get_control_plane_cert_key(&self) -> BoxFuture<'_, Result<Option<(String, String)>>> {
        Box::pin(async move {
            let (query, values) = Query::select()
                .columns([Certs::Key, Certs::Value])
                .from(Certs::Table)
                .and_where(Expr::col(Certs::Key).is_in([Certs::key(), Certs::cert()]))
                .build_any_sqlx(&*self.query_builder);
            let rows = sqlx::query_with(&query, values)
                .fetch_all(&self.pool)
                .await?;
            let maybe_cert_key = rows.into_iter().fold((None, None), |(cert, key), row| {
                let row_key = row.get::<String, _>(0);
                let row_value = row.get::<String, _>(1);
                match &row_key {
                    row_key if row_key == Certs::key() => (cert, Some(row_value)),
                    row_key if row_key == Certs::cert() => (Some(row_value), key),
                    row_key => unreachable!("unexpected key {row_key}"),
                }
            });
            match maybe_cert_key {
                (Some(cert), Some(key)) => Ok(Some((cert, key))),
                (None, None) => Ok(None),
                (None, _) => Err(anyhow::anyhow!("cert is missing"))?,
                (_, None) => Err(anyhow::anyhow!("key is missing"))?,
            }
        })
    }

    fn store_control_plane_cert_key<'a>(
        &'a self,
        key: &'a str,
        cert: &'a str,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let (query, values) = Query::insert()
                .into_table(Certs::Table)
                .columns([Certs::Key, Certs::Value, Certs::CreatedAt])
                .values_panic([
                    Certs::key().into(),
                    key.into(),
                    Expr::current_timestamp().into(),
                ])
                .values_panic([
                    Certs::cert().into(),
                    cert.into(),
                    Expr::current_timestamp().into(),
                ])
                .build_any_sqlx(&*self.query_builder);
            sqlx::query_with(&query, values).execute(&self.pool).await?;
            Ok(())
        })
    }

    fn store_daemon_token<'a>(&'a self, token: &'a DaemonToken) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let DaemonToken {
                id,
                secret,
                issued_at,
                used_at,
            } = token;
            let (query, values) = Query::insert()
                .into_table(DaemonTokens::Table)
                .columns([
                    DaemonTokens::Id,
                    DaemonTokens::Secret,
                    DaemonTokens::IssuedAt,
                    DaemonTokens::UsedAt,
                ])
                .values_panic([
                    (*id).into(),
                    secret.into(),
                    (*issued_at).into(),
                    (*used_at).into(),
                ])
                .build_any_sqlx(&*self.query_builder);
            sqlx::query_with(&query, values).execute(&self.pool).await?;
            Ok(())
        })
    }

    fn list_daemon_tokens(&self) -> BoxFuture<'_, Result<Vec<DaemonToken>>> {
        Box::pin(async move {
            let (query, values) = Query::select()
                .columns([
                    DaemonTokens::Id,
                    DaemonTokens::Secret,
                    DaemonTokens::IssuedAt,
                    DaemonTokens::UsedAt,
                ])
                .from(DaemonTokens::Table)
                .build_any_sqlx(&*self.query_builder);
            let tokens = sqlx::query_with(&query, values)
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .map(|row| DaemonToken {
                    id: row.get(0),
                    secret: row.get(1),
                    issued_at: row.get(2),
                    used_at: row.get(3),
                })
                .collect();
            Ok(tokens)
        })
    }

    fn delete_daemon_token(&self, id: Uuid) -> BoxFuture<'_, Result<()>> {
        Box::pin(async move {
            let (query, values) = Query::delete()
                .from_table(DaemonTokens::Table)
                .and_where(Expr::col(DaemonTokens::Id).eq(id))
                .build_any_sqlx(&*self.query_builder);
            sqlx::query_with(&query, values).execute(&self.pool).await?;
            Ok(())
        })
    }

    fn consume_token(&self, id: Uuid) -> BoxFuture<'_, Result<Option<DaemonToken>>> {
        Box::pin(async move {
            let mut conn = self.pool.acquire().await?;
            let mut transaction = conn.begin().await?;
            let (query, values) = Query::select()
                .columns([
                    DaemonTokens::Id,
                    DaemonTokens::Secret,
                    DaemonTokens::IssuedAt,
                    DaemonTokens::UsedAt,
                ])
                .from(DaemonTokens::Table)
                .and_where(Expr::col(DaemonTokens::Id).eq(id))
                .lock_exclusive()
                .build_any_sqlx(&*self.query_builder);
            let token = sqlx::query_with(&query, values)
                .fetch_optional(&mut *transaction)
                .await?
                .map(|row| DaemonToken {
                    id: row.get(0),
                    secret: row.get(1),
                    issued_at: row.get(2),
                    used_at: row.get(3),
                });
            let token = match token {
                None => return Ok(None),
                Some(token) if token.used_at.is_none() => token,
                Some(DaemonToken { id, .. }) => Err(AppError::token_used(id))?,
            };
            // update token
            let (query, values) = Query::update()
                .table(DaemonTokens::Table)
                .values([(DaemonTokens::UsedAt, Expr::current_timestamp().into())])
                .and_where(Expr::col(DaemonTokens::Id).eq(id))
                .build_any_sqlx(&*self.query_builder);

            sqlx::query_with(&query, values)
                .execute(&mut *transaction)
                .await?;
            transaction.commit().await?;
            Ok(Some(token))
        })
    }

    fn add_daemon(&self, id: Uuid) -> BoxFuture<'_, Result<()>> {
        Box::pin(async move {
            let (query, values) = Query::insert()
                .columns([Daemons::Id, Daemons::JoinedAt])
                .into_table(Daemons::Table)
                .values_panic([id.into(), Expr::current_timestamp().into()])
                .build_any_sqlx(&*self.query_builder);
            sqlx::query_with(&query, values).execute(&self.pool).await?;
            Ok(())
        })
    }
    
    fn get_daemon(&self, id: Uuid) -> BoxFuture<'_, Result<Option<Daemon>>> {
        Box::pin(async move {
            let (query, values) = Query::select()
                .columns([
                    Daemons::Id,
                    Daemons::DisplayName,
                    Daemons::Address,
                    Daemons::LastOnline,
                    Daemons::JoinedAt,
                ])
                .from(Daemons::Table)
                .and_where(Expr::col(Daemons::Id).eq(id))
                .build_any_sqlx(&*self.query_builder);
            Ok(
                sqlx::query_with(&query, values).fetch_optional(&self.pool).await?
                    .map(|row| Daemon {
                        id: row.get(0),
                        name: row.get(1),
                        address: row.get(2),
                        last_seen: row.get(3),
                        joined_at: row.get(4),
                        status: Default::default(),
                    })
            )
        })
    }

    fn delete_daemon(&self, id: Uuid) -> BoxFuture<'_, Result<()>> {
        Box::pin(async move {
            let (query, values) = Query::delete()
                .from_table(Daemons::Table)
                .and_where(Expr::col(Daemons::Id).eq(id))
                .build_any_sqlx(&*self.query_builder);
            sqlx::query_with(&query, values).execute(&self.pool).await?;
            Ok(())
        })
    }

    fn daemon_set_last_seen(
        &self,
        id: Uuid,
        last_seen: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<()>> {
        Box::pin(async move {
            let (query, values) = Query::update()
                .table(Daemons::Table)
                .values([(Daemons::LastOnline, last_seen.into())])
                .and_where(Expr::col(Daemons::Id).eq(id))
                .build_any_sqlx(&*self.query_builder);
            sqlx::query_with(&query, values).execute(&self.pool).await?;
            Ok(())
        })
    }

    fn list_daemons(&self) -> BoxFuture<'_, Result<Vec<Daemon>>> {
        Box::pin(async move {
            let (query, values) = Query::select()
                .columns([
                    Daemons::Id,
                    Daemons::DisplayName,
                    Daemons::Address,
                    Daemons::LastOnline,
                    Daemons::JoinedAt,
                ])
                .from(Daemons::Table)
                .build_any_sqlx(&*self.query_builder);
            let daemons = sqlx::query_with(&query, values)
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .map(|row| Daemon {
                    id: row.get(0),
                    name: row.get(1),
                    address: row.get(2),
                    last_seen: row.get(3),
                    joined_at: row.get(4),
                    status: Default::default(),
                })
                .collect();
            Ok(daemons)
        })
    }

    fn get_daemon_graph(&self, id: Uuid) -> BoxFuture<'_, Result<DaemonGraph>> {
        Box::pin(async move {
            let (query, values) = Query::select()
                .columns([Nodes::Id, Nodes::Config])
                .from(Nodes::Table)
                .and_where(Expr::col(Nodes::DaemonId).eq(id))
                .build_any_sqlx(&*self.query_builder);
            let nodes = sqlx::query_with(&query, values)
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .map(|row| {
                    let config = row.get::<Json<_>, _>(1).0;
                    Ok(DaemonNode {
                        id: row.get(0),
                        config: serde_json::from_value(config)?,
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
            Ok(DaemonGraph { nodes, edges })
        })
    }

    fn set_daemon_name<'a>(&'a self, id: Uuid, name: Option<&'a str>) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let (query, values) = Query::update()
                .table(Daemons::Table)
                .values([(Daemons::DisplayName, name.into())])
                .and_where(Expr::col(Daemons::Id).eq(id))
                .build_any_sqlx(&*self.query_builder);
            sqlx::query_with(&query, values).execute(&self.pool).await?;
            Ok(())
        })
    }
}
