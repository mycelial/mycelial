use std::{
    borrow::Cow,
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use crate::{migration, model, workspace, Result};
use axum::{async_trait, body::Bytes};
use chrono::{DateTime, NaiveDateTime, Utc};
use common::PipeConfig;
use futures::{future::BoxFuture, stream::BoxStream, StreamExt};
use sea_query::{
    Expr, Iden, OnConflict, Order, PostgresQueryBuilder, Query, QueryBuilder, SchemaBuilder, SqliteQueryBuilder
};
use sea_query_binder::{SqlxBinder, SqlxValues};
use sqlx::{
    database::HasArguments, migrate::Migrate, types::Json, ColumnIndex, Connection, Database,
    Executor, IntoArguments, PgPool, Pool, Postgres, Row, Sqlite, SqlitePool, Transaction,
};
use uuid::Uuid;

// FIXME: pool options and configurable pool size
pub async fn new(url: &str) -> Result<Box<dyn DbTrait>> {
    let mut url = url::Url::parse(url)?;
    let mut params: HashMap<Cow<str>, Cow<str>> = url.query_pairs().collect();
    let db: Box<dyn DbTrait> = match url.scheme() {
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
            url.set_query(Some(&query));
            Box::new(
                Db::<Sqlite>::new(
                    url.as_ref(),
                    Box::new(SqliteQueryBuilder),
                    Box::new(SqliteQueryBuilder),
                )
                .await?,
            )
        }
        "postgres" => Box::new(
            Db::<Postgres>::new(
                url.as_ref(),
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

#[derive(Iden)]
enum Messages {
    Table,
    Id,
    Topic,
    Origin,
    StreamType,
    CreatedAt,
}

#[derive(Iden)]
enum MessageChunks {
    Table,
    MessageId,
    ChunkId,
    Data,
}

#[derive(Iden)]
enum Workspaces {
    Table,
    Id,
    UserId,
    Name,
    CreatedAt,
}

#[derive(Iden)]
enum UserDaemonTokens {
    Table,
    UserId,
    DaemonToken,
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

// FIXME: auto-derive trait from impl?
#[async_trait]
pub trait DbTrait: Send + Sync {
    async fn migrate(&self) -> Result<()>;

    async fn provision_daemon(
        &self,
        unique_id: &str,
        user_id: &str,
        display_name: &str,
        unique_client_id: &str,
        client_secret_hash: &str,
    ) -> Result<()>;

    async fn submit_sections(
        &self,
        unique_id: &str,
        user_id: &str,
        sourses: &serde_json::Value,
        destinations: &serde_json::Value,
    ) -> Result<()>;

    async fn insert_config(
        &self,
        config: &serde_json::Value,
        workspace_id: i32,
        user_id: &str,
    ) -> Result<i64>;

    async fn update_config(&self, id: i64, config: &serde_json::Value, user_id: &str)
        -> Result<()>;

    async fn delete_config(&self, id: i64, user_id: &str) -> Result<()>;

    async fn get_config(&self, id: i64, user_id: &str) -> Result<PipeConfig>;

    async fn get_configs(&self, user_id: &str) -> Result<Vec<PipeConfig>>;

    async fn get_clients(&self, user_id: &str) -> Result<model::Clients>;

    async fn get_workspaces(&self, user_id: &str) -> Result<Vec<model::Workspace>>;

    async fn create_workspace(
        &self,
        workspace: model::Workspace,
        user_id: &str,
    ) -> Result<model::Workspace>;

    async fn get_workspace(&self, id: i32, user_id: &str) -> Result<Option<model::Workspace>>;

    async fn update_workspace(
        &self,
        workspace: model::Workspace,
        user_id: &str,
    ) -> Result<model::Workspace>;

    async fn delete_workspace(&self, id: i32, user_id: &str) -> Result<()>;

    async fn get_user_id_for_daemon_token(&self, token: &str) -> Result<Option<String>>;

    async fn get_user_daemon_token(&self, user_id: &str) -> Result<Option<String>>;

    async fn rotate_user_daemon_token(&self, user_id: &str, token: &str) -> Result<()>;

    async fn get_user_id_and_secret_hash(&self, user_id: &str) -> Result<Option<(String, String)>>;

    async fn ingest_message(
        &self,
        topic: &str,
        origin: &str,
        stream_type: &str,
        stream: BoxStream<'_, Result<Vec<u8>>>,
    ) -> Result<()>;

    async fn stream_message(
        &self,
        topic: &str,
        offset: i64,
    ) -> Result<Option<model::MessageStream>>;
}

#[async_trait]
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
    async fn migrate(&self) -> Result<()> {
        migration::migrate(&self.pool, &*self.schema_builder).await?;
        Ok(())
    }

    async fn provision_daemon(
        &self,
        unique_id: &str,
        user_id: &str,
        display_name: &str,
        unique_client_id: &str,
        client_secret_hash: &str,
    ) -> Result<()> {
        let (query, values) = Query::insert()
            .columns([
                Clients::Id,
                Clients::UserId,
                Clients::DisplayName,
                Clients::Sources,
                Clients::Destinations,
                Clients::UniqueClientId,
                Clients::ClientSecretHash,
            ])
            .into_table(Clients::Table)
            .values_panic([
                unique_id.into(),
                user_id.into(),
                display_name.into(),
                serde_json::json!([]).into(),
                serde_json::json!([]).into(),
                unique_client_id.into(),
                client_secret_hash.into(),
            ])
            .on_conflict(
                OnConflict::new()
                    .expr(Expr::col(Clients::Id))
                    .update_columns([
                        Clients::DisplayName,
                        Clients::Sources,
                        Clients::Destinations,
                        Clients::UniqueClientId,
                        Clients::ClientSecretHash,
                    ])
                    .to_owned(),
            )
            .build_any_sqlx(&*self.query_builder);
        tracing::debug!("{query}");
        sqlx::query_with(&query, values).execute(&self.pool).await?;
        Ok(())
    }

    async fn submit_sections(
        &self,
        unique_id: &str,
        user_id: &str,
        sources: &serde_json::Value,
        destinations: &serde_json::Value,
    ) -> Result<()> {
        let (query, values) = Query::update()
            .table(Clients::Table)
            .values([
                (Clients::Sources, sources.clone().into()),
                (Clients::Destinations, destinations.clone().into()),
            ])
            .and_where(Expr::col(Clients::Id).eq(unique_id))
            .and_where(Expr::col(Clients::UserId).eq(user_id))
            .build_any_sqlx(&*self.query_builder);
        tracing::debug!("{query}");
        sqlx::query_with(&query, values).execute(&self.pool).await?;
        Ok(())
    }

    async fn insert_config(
        &self,
        config: &serde_json::Value,
        workspace_id: i32,
        user_id: &str,
    ) -> Result<i64> {
        let (query, values) = Query::insert()
            .into_table(Pipes::Table)
            .columns([Pipes::RawConfig, Pipes::WorkspaceId, Pipes::UserId])
            // FIXME: config cloned
            .values_panic([config.clone().into(), workspace_id.into(), user_id.into()])
            .returning(Query::returning().column(Pipes::Id))
            .build_any_sqlx(&*self.query_builder);
        tracing::debug!("{query}");
        let id = sqlx::query_with(&query, values)
            .fetch_one(&self.pool)
            .await
            .map(|row| row.get(0))?;
        Ok(id)
    }

    async fn update_config(
        &self,
        id: i64,
        config: &serde_json::Value,
        user_id: &str,
    ) -> Result<()> {
        let (query, values) = Query::update()
            .table(Pipes::Table)
            .values([(Pipes::RawConfig, config.clone().into())])
            .and_where(Expr::col(Pipes::Id).eq(id))
            .and_where(Expr::col(Pipes::UserId).eq(user_id))
            .build_any_sqlx(&*self.query_builder);
        tracing::debug!("{query}");
        sqlx::query_with(&query, values).execute(&self.pool).await?;
        Ok(())
    }

    async fn delete_config(&self, id: i64, user_id: &str) -> Result<()> {
        let (query, values) = Query::delete()
            .from_table(Pipes::Table)
            .and_where(Expr::col(Pipes::Id).eq(id))
            .and_where(Expr::col(Pipes::UserId).eq(user_id))
            .build_any_sqlx(&*self.query_builder);
        tracing::debug!("{query}");
        sqlx::query_with(&query, values).execute(&self.pool).await?;
        Ok(())
    }

    async fn get_config(&self, id: i64, user_id: &str) -> Result<PipeConfig> {
        unimplemented!()
    }

    async fn get_configs(&self, user_id: &str) -> Result<Vec<PipeConfig>> {
        let (query, values) = Query::select()
            .columns([Pipes::Id, Pipes::WorkspaceId, Pipes::RawConfig])
            .from(Pipes::Table)
            .and_where(Expr::col(Pipes::UserId).eq(user_id))
            .build_any_sqlx(&*self.query_builder);
        let configs = sqlx::query_with(&query, values)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| PipeConfig {
                id: row.get::<i64, _>(0) as u64,
                workspace_id: row.get(1),
                pipe: row.get(2),
            })
            .collect();
        Ok(configs)
    }

    async fn get_clients(&self, user_id: &str) -> Result<model::Clients> {
        // todo: should we return ui as client?
        let (query, values) = Query::select()
            .columns([
                Clients::Id,
                Clients::DisplayName,
                Clients::Sources,
                Clients::Destinations,
            ])
            .from(Clients::Table)
            .and_where(Expr::col(Clients::UserId).eq(user_id))
            .build_any_sqlx(&*self.query_builder);
        tracing::debug!("{query}");
        let clients = sqlx::query_with(&query, values)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| model::Client {
                id: row.get(0),
                display_name: row.get(1),
                sources: serde_json::from_value(row.get::<serde_json::Value, _>(2)).unwrap(),
                destinations: serde_json::from_value(row.get::<serde_json::Value, _>(3)).unwrap(),
            })
            .collect();
        Ok(model::Clients { clients })
    }

    async fn get_workspaces(&self, user_id: &str) -> Result<Vec<model::Workspace>> {
        let (query, values) = Query::select()
            .columns([Workspaces::Id, Workspaces::Name, Workspaces::CreatedAt])
            .from(Workspaces::Table)
            .and_where(Expr::col(Workspaces::UserId).eq(user_id.to_string()))
            .build_any_sqlx(&*self.query_builder);
        tracing::debug!("{query}");
        let workspaces = sqlx::query_with(&query, values)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| {
                Ok(model::Workspace {
                    id: row.get(0),
                    name: row.get(1),
                    created_at: row.get(2),
                    pipe_configs: vec![],
                })
            })
            .collect::<Result<Vec<model::Workspace>, anyhow::Error>>()?;
        Ok(workspaces)
    }

    async fn create_workspace(
        &self,
        mut workspace: model::Workspace,
        user_id: &str,
    ) -> Result<model::Workspace> {
        let (query, values) = Query::insert()
            .into_table(Workspaces::Table)
            .columns([Workspaces::Name, Workspaces::UserId])
            .values_panic([workspace.name.as_str().into(), user_id.into()])
            .returning(Query::returning().columns([Workspaces::Id, Workspaces::CreatedAt]))
            .build_any_sqlx(&*self.query_builder);
        tracing::debug!("{query}");
        let result = sqlx::query_with(&query, values)
            .fetch_one(&self.pool)
            .await?;
        workspace.id = result.get(0);
        workspace.created_at = result.get(1);
        Ok(workspace)
    }

    async fn get_workspace(&self, id: i32, user_id: &str) -> Result<Option<model::Workspace>> {
        // FIXME: use join
        let (query, values) = Query::select()
            .columns([Pipes::Id, Pipes::WorkspaceId, Pipes::RawConfig])
            .from(Pipes::Table)
            .and_where(Expr::col(Pipes::WorkspaceId).eq(id))
            .and_where(Expr::col(Pipes::UserId).eq(user_id))
            .build_any_sqlx(&*self.query_builder);
        tracing::debug!("{query}");
        let pipes = sqlx::query_with(&query, values)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| PipeConfig {
                id: row.get::<i64, _>(0) as u64,
                workspace_id: row.get::<i32, _>(1),
                pipe: row.get(2),
            })
            .collect();

        let (query, values) = Query::select()
            .columns([Workspaces::Id, Workspaces::Name, Workspaces::CreatedAt])
            .from(Workspaces::Table)
            .and_where(Expr::col(Workspaces::Id).eq(id))
            .and_where(Expr::col(Workspaces::UserId).eq(user_id))
            .build_any_sqlx(&*self.query_builder);
        tracing::debug!("{query}");
        let workspace = sqlx::query_with(&query, values)
            .fetch_optional(&self.pool)
            .await
            .map(move |maybe_row| {
                maybe_row.map(|row| model::Workspace {
                    id: row.get(0),
                    name: row.get(1),
                    created_at: row.get(2),
                    pipe_configs: pipes,
                })
            })?;
        Ok(workspace)
    }

    async fn update_workspace(
        &self,
        workspace: model::Workspace,
        user_id: &str,
    ) -> Result<model::Workspace> {
        unimplemented!()
    }

    async fn delete_workspace(&self, id: i32, user_id: &str) -> Result<()> {
        let (query, values) = Query::delete()
            .from_table(Workspaces::Table)
            .and_where(Expr::col(Workspaces::Id).eq(id))
            .and_where(Expr::col(Workspaces::UserId).eq(user_id))
            .build_any_sqlx(&*self.query_builder);
        tracing::debug!("{query}");
        sqlx::query_with(&query, values).execute(&self.pool).await?;
        Ok(())
    }

    async fn get_user_id_for_daemon_token(&self, token: &str) -> Result<Option<String>> {
        let (query, values) = Query::select()
            .columns([UserDaemonTokens::UserId])
            .from(UserDaemonTokens::Table)
            .and_where(Expr::col(UserDaemonTokens::DaemonToken).eq(token))
            .build_any_sqlx(&*self.query_builder);
        tracing::debug!("{query}");
        Ok(sqlx::query_with(&query, values)
            .fetch_optional(&self.pool)
            .await
            .map(|maybe_row| maybe_row.map(|row| row.get(0)))?)
    }

    async fn get_user_daemon_token(&self, user_id: &str) -> Result<Option<String>> {
        let (query, values) = Query::select()
            .columns([UserDaemonTokens::DaemonToken])
            .from(UserDaemonTokens::Table)
            .and_where(Expr::col(UserDaemonTokens::UserId).eq(user_id))
            .build_any_sqlx(&*self.query_builder);
        tracing::debug!("{query}");
        Ok(sqlx::query_with(&query, values)
            .fetch_optional(&self.pool)
            .await
            .map(|row| row.map(|row| row.get(0)))?)
    }

    async fn rotate_user_daemon_token(&self, user_id: &str, token: &str) -> Result<()> {
        let (query, values) = Query::insert()
            .into_table(UserDaemonTokens::Table)
            .columns([UserDaemonTokens::UserId, UserDaemonTokens::DaemonToken])
            .values_panic([user_id.into(), token.into()])
            .on_conflict(
                OnConflict::new()
                    .expr(Expr::col(UserDaemonTokens::UserId))
                    .update_column(UserDaemonTokens::DaemonToken)
                    .to_owned(),
            )
            .build_any_sqlx(&*self.query_builder);
        sqlx::query_with(&query, values).execute(&self.pool).await?;
        Ok(())
    }

    async fn get_user_id_and_secret_hash(&self, user_id: &str) -> Result<Option<(String, String)>> {
        let (query, values) = Query::select()
            .columns([Clients::UserId, Clients::ClientSecretHash])
            .from(Clients::Table)
            .and_where(Expr::col(Clients::UniqueClientId).eq(user_id))
            .build_any_sqlx(&*self.query_builder);
        tracing::debug!("{query}");
        Ok(sqlx::query_with(&query, values)
            .fetch_optional(&self.pool)
            .await
            .map(|maybe_row| maybe_row.map(|row| (row.get(0), row.get(1))))?)
    }

    async fn ingest_message(
        &self,
        topic: &str,
        origin: &str,
        stream_type: &str,
        mut stream: BoxStream<'_, Result<Vec<u8>>>,
    ) -> Result<()> {
        let mut connection = self.pool.acquire().await?;
        let mut transaction = connection.as_mut().begin().await?;
        let (query, values) = Query::insert()
            .columns([Messages::Topic, Messages::Origin, Messages::StreamType])
            .into_table(Messages::Table)
            .values_panic([topic.into(), origin.into(), stream_type.into()])
            .returning(Query::returning().column(Messages::Id))
            .build_any_sqlx(&*self.query_builder);
        tracing::debug!("{query}");
        let message_id: i64 = sqlx::query_with(&query, values)
            .fetch_one(&mut *transaction)
            .await?
            .get(0);
        let (query, _) = Query::insert()
            .columns([
                MessageChunks::MessageId,
                MessageChunks::ChunkId,
                MessageChunks::Data,
            ])
            .into_table(MessageChunks::Table)
            // values are used here to just generate placeholders
            // sqlx binder will always return sql and values separately
            .values_panic([0_i64.into(), 0_i32.into(), Vec::<u8>::new().into()])
            .build_any_sqlx(&*self.query_builder);
        tracing::debug!("{query}");
        let mut chunk_id = 0;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let values = sea_query::Values(vec![message_id.into(), chunk_id.into(), chunk.into()]);
            sqlx::query_with(&query, SqlxValues(values))
                .execute(&mut *transaction)
                .await?;
            chunk_id += 1;
        }
        transaction.commit().await?;
        Ok(())
    }

    async fn stream_message(
        &self,
        topic: &str,
        offset: i64,
    ) -> Result<Option<model::MessageStream>> {
        let (query, values) = Query::select()
            .columns([Messages::Id, Messages::Origin, Messages::StreamType])
            .from(Messages::Table)
            .and_where(Expr::col(Messages::Id).gt(offset))
            .and_where(Expr::col(Messages::Topic).eq(topic))
            .limit(1)
            .build_any_sqlx(&*self.query_builder);
        let (id, origin, stream_type) = match sqlx::query_with(&query, values)
            .fetch_optional(&self.pool)
            .await?
        {
            None => return Ok(None),
            Some(row) => (
                row.get::<i64, _>(0),
                row.get::<String, _>(1),
                row.get::<String, _>(2),
            ),
        };

        let mut connection = self.pool.acquire().await?;
        // move connection into stream wrapper around sqlx's stream
        let (query, values) = Query::select()
            .column(MessageChunks::Data)
            .from(MessageChunks::Table)
            .and_where(Expr::col(MessageChunks::MessageId).eq(id))
            .order_by(MessageChunks::ChunkId, Order::Asc)
            .build_any_sqlx(&*self.query_builder);
        tracing::debug!("{query}");
        let stream = async_stream::stream! {
            let mut stream = sqlx::query_with(&query, values)
                .fetch(&mut *connection)
                .map(|maybe_row| {
                    maybe_row
                        .map(|row| row.get::<Vec<u8>, _>(0))
                        .map_err(Into::into)
                });
            while let Some(chunk) = stream.next().await {
                yield chunk;
            }
        };
        Ok(Some(model::MessageStream {
            id,
            origin,
            stream_type,
            stream: Box::pin(stream),
        }))
    }
}
