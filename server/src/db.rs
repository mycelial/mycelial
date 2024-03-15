use std::{pin::Pin, str::FromStr};

use anyhow::Result;
use chrono::{DateTime, Utc};
use common::{Destination, PipeConfig, PipeConfigs, Source};
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use sqlx::{
    pool::{Pool, PoolConnection, PoolOptions},
    Row,
    Transaction
};

pub struct DB<D>{
    pool: Pool<D>,
}

impl DB {
    pub async fn try_new(database_url: &str) -> Result<Self> {
        let pool = PoolOptions::new()
            .min_connections(5)
            .max_connections(5)
            .connect(database_url)
            .await?;

        //sqlx::migrate!().run(&pool).await?;
        Ok(Self {
            pool
        })
    }
    
    pub async fn get_connection(&self) -> Result<PoolConnection<sqlx::Any>> {
        Ok(self.pool.acquire().await?)
    }
    #[allow(clippy::too_many_arguments)]
    pub async fn provision_daemon(
        &self,
        unique_id: &str,
        user_id: &str,
        display_name: &str,
        unique_client_id: &str,
        client_secret_hash: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO clients (id, user_id, display_name, sources, destinations, unique_client_id, client_secret_hash) \
            VALUES ($1, $2, $3, '[]', '[]', $4, $5) \
            ON CONFLICT (id) DO UPDATE SET \
            display_name=excluded.display_name, \
            sources=excluded.sources, \
            destinations=excluded.destinations, \
            unique_client_id=excluded.unique_client_id, \
            client_secret_hash=excluded.client_secret_hash"
        )
            .bind(unique_id)
            .bind(user_id)
            .bind(display_name)
            .bind(unique_client_id)
            .bind(client_secret_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn submit_sections(
        &self,
        unique_id: &str,
        user_id: &str,
        sources: &str,
        destinations: &str,
    ) -> Result<()> {
        sqlx::query("UPDATE clients SET sources=$1, destinations=$2 WHERE id=$3 AND user_id=$4")
            .bind(sources)
            .bind(destinations)
            .bind(unique_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn insert_config(
        &self,
        config: &serde_json::Value,
        workspace_id: i32,
        user_id: &str,
    ) -> Result<u64> {
        let config: String = serde_json::to_string(config)?;
        let result =
            sqlx::query("INSERT INTO pipes (raw_config, workspace_id, user_id) VALUES ($1::json, $2, $3) RETURNING id")
                .bind(config)
                .bind(workspace_id)
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?;
        let id = result.get::<i32, _>(0);
        Ok(id.try_into().unwrap())
    }

    pub async fn update_config(
        &self,
        id: u64,
        config: &serde_json::Value,
        user_id: &str,
    ) -> Result<()> {
        let config: String = serde_json::to_string(config)?;
        // FIXME: unwrap
        let id: i32 = id.try_into().unwrap();
        let _ =
            sqlx::query("update pipes set raw_config = $1::json WHERE id = $2 and user_id = $3")
                .bind(config)
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        Ok(())
    }

    pub async fn delete_config(&self, id: u64, user_id: &str) -> Result<()> {
        // FIXME: unwrap
        let id: i32 = id.try_into().unwrap();
        let _ = sqlx::query("DELETE FROM pipes WHERE id = $1 and user_id = $2")
            .bind(id) // fixme
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn new_message(
        &self,
        transaction: &mut Transaction<'_, sqlx::Any>,
        topic: &str,
        origin: &str,
        stream_type: &str,
    ) -> Result<i32> {
        unimplemented!()
      //let id = sqlx::query(
      //    "INSERT INTO messages(topic, origin, stream_type) VALUES($1, $2, $3) RETURNING ID",
      //)
      //.bind(topic)
      //.bind(origin)
      //.bind(stream_type)
      //.fetch_one(&mut *transaction)
      //.await
      //.map(|row| row.get::<i32, _>(0))?;
      //Ok(id)
    }

    pub async fn store_chunk(
        &self,
        transaction: &mut Transaction<'_, sqlx::Any>,
        message_id: i32,
        bytes: &[u8],
    ) -> Result<()> {
        unimplemented!()
      //sqlx::query("INSERT INTO records (message_id, data) VALUES ($1, $2)")
      //    .bind(message_id)
      //    .bind(bytes)
      //    .execute(&mut *transaction)
      //    .await?;
      //Ok(())
    }

    pub async fn get_message(
        &self,
        topic: &str,
        offset: u64,
    ) -> Result<Option<MessageStream>> {
        let mut connection = self.pool.acquire().await?;
        let offset: i64 = offset.try_into().unwrap();
        let message_info = sqlx::query(
            "SELECT id, origin, stream_type FROM messages WHERE id > $1 and topic = $2 LIMIT 1",
        )
            .bind(offset)
            .bind(topic)
            .fetch_optional(&mut *connection)
            .await?
            .map(|row| {
                (
                    row.get::<i64, _>(0) as u64,
                    row.get::<String, _>(1),
                    row.get::<String, _>(2),
                )
            });

        let (id, origin, stream_type) = match message_info {
            Some((id, o, t)) => (id, o, t),
            None => return Ok(None),
        };

        // move connection into stream wrapper around sqlx's stream
        let stream = async_stream::stream! {
            let mut stream = sqlx::query("SELECT data FROM records r WHERE r.message_id = $1")
                .bind(id as i64)
                .fetch(&mut *connection);
            while let Some(row) = stream.next().await {
                yield row
                    .map(|row| row.get::<Vec<u8>, &str>("data"))
                    .map_err(Into::into);
            }
        };
        Ok(Some(MessageStream {
            id,
            origin,
            stream_type,
            stream: Box::pin(stream),
        }))
    }

    pub async fn get_clients(&self, user_id: &str) -> Result<Clients> {
        // todo: should we return ui as client?
        let mut query = sqlx::query("SELECT id, display_name, sources, destinations FROM clients");
        if cfg!(feature = "require_auth") {
            query = sqlx::query(
                "SELECT id, display_name, sources, destinations FROM clients where user_id = $1",
            )
            .bind(user_id);
        }
        let rows = query.fetch_all(&self.pool).await?;

        let mut clients: Vec<Client> = Vec::new();
        for row in rows.iter() {
            let id = row.get("id");
            let display_name = row.get("display_name");
            let sources: Option<String> = row.get("sources");
            let sources = serde_json::from_str(&sources.unwrap_or("[]".to_string()))?;
            let destinations: Option<String> = row.get("destinations");
            let destinations = serde_json::from_str(&destinations.unwrap_or("[]".to_string()))?;
            clients.push(Client {
                id,
                display_name,
                sources,
                destinations,
            });
        }
        Ok(Clients { clients })
    }

    pub async fn get_workspaces(&self, user_id: &str) -> Result<Vec<Workspace>> {
        let workspaces = sqlx::query(r"SELECT id, name, created_at FROM workspaces where user_id = $1")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| {
                Ok(Workspace {
                    id: row.get(0),
                    name: row.get(1),
                    created_at: DateTime::from_str(&row.get::<String, _>(2))?,
                    pipe_configs: vec![],
                })
            })
            .collect::<Result<Vec<Workspace>, anyhow::Error>>()?;
        Ok(workspaces)
    }

    pub async fn create_workspace(
        &self,
        mut workspace: Workspace,
        user_id: &str,
    ) -> Result<Workspace> {
        let result =
            sqlx::query("INSERT INTO workspaces (name, user_id) VALUES ($1, $2) RETURNING id")
                .bind(workspace.name.as_str())
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?;
        let id = result.get::<i32, _>(0);
        workspace.id = id;
        workspace.created_at = Utc::now();
        Ok(workspace)
    }

    pub async fn get_workspace(&self, id: u64, user_id: &str) -> Result<Workspace> {
        // FIXME: use join
        let pipes = sqlx::query(
            "SELECT id, raw_config, workspace_id from pipes where workspace_id = $1 and user_id = $2",
        )
            .bind(id as i64)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| {
                PipeConfig {
                    id: row.get::<i64, _>(0) as u64,
                    workspace_id: row.get::<i64, _>(1) as u64,
                    pipe: serde_json::from_str(&row.get::<String, _>(2)).unwrap(),
                }
            })
            .collect();

        let row = sqlx::query(
            r"SELECT id, name, created_at FROM workspaces WHERE id = $1 and user_id = $2",
        )
            .bind(id as i64)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map(|row| {
                row
            })?;


        let workspace = Workspace{ 
            id: row.get(0),
            name: row.get(1),
            created_at: DateTime::from_str(&row.get::<String, _>(2))?, 
            pipe_configs: pipes
        };
        Ok(workspace)
    }

    pub async fn update_workspace(
        &self,
        workspace: Workspace,
        user_id: &str,
    ) -> Result<Workspace> {
        let _ = sqlx::query("UPDATE workspaces SET name = $1 where id = $2 and user_id = $3")
            .bind(workspace.name.clone())
            .bind(workspace.id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(workspace)
    }

    pub async fn delete_workspace(&self, id: u64, user_id: &str) -> Result<()> {
        let id: i64 = id.try_into().unwrap();
        let _ = sqlx::query("DELETE FROM workspaces WHERE id = $1 and user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_config(&self, id: u64, user_id: &str) -> Result<PipeConfig> {
        let row = sqlx::query("SELECT id, workspace_id, raw_config from pipes WHERE id = $1 and user_id = $2")
            .bind(id as i64)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(PipeConfig {
            id: row.get::<i64, _>(0) as u64,
            workspace_id: row.get::<i64, _>(1) as u64,
            pipe: serde_json::from_str(&row.get::<String, _>(2)).unwrap(),
        })
    }

    pub async fn get_configs(&self, user_id: &str) -> Result<PipeConfigs> {
        let configs = sqlx::query("SELECT id, raw_config, workspace_id from pipes where user_id = $1")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row: AnyRow| {
                PipeConfig {
                    id: row.get::<i64, _>(0) as u64,
                    workspace_id: row.get::<i64, _>(1) as u64,
                    pipe: serde_json::from_str(&row.get::<String, _>(2)).unwrap(),
                }
            })
            .collect();
        Ok(PipeConfigs { configs })
    }

    pub async fn get_user_id_for_daemon_token(&self, token: &str) -> Result<String> {
        let user_id: String =
            sqlx::query("SELECT user_id FROM user_daemon_tokens WHERE daemon_token = $1")
                .bind(token)
                .fetch_one(&self.pool)
                .await
                .map(|row| row.get(0))?;
        Ok(user_id)
    }

    pub async fn get_user_daemon_token(&self, user_id: &str) -> Result<String> {
        let daemon_token =
            sqlx::query("SELECT daemon_token FROM user_daemon_tokens WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?
                .get::<String, _>(0);
        Ok(daemon_token)
    }

    pub async fn rotate_user_daemon_token(
        &self,
        user_id: &str,
        new_token: &str,
    ) -> Result<()> {
        // todo: Should this schema have "deleted_at" and then we only insert rows?
        sqlx::query(
            "INSERT INTO user_daemon_tokens (user_id, daemon_token) VALUES ($1, $2) ON CONFLICT (user_id) DO UPDATE SET daemon_token = $2"
        )
        .bind(user_id)
        .bind(new_token)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_user_id_and_secret_hash(
        &self,
        client_id: &str,
    ) -> Result<(String, String)> {
        let (user_id, client_secret_hash): (String, String) = sqlx::query(
            "SELECT user_id, client_secret_hash FROM clients WHERE unique_client_id = $1",
        )
        .bind(client_id)
        .fetch_one(&self.pool)
        .await
        .map(|row| (row.get(0), row.get(1)))?;
        Ok((user_id, client_secret_hash))
    }
}
