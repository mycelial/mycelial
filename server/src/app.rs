use std::path::Path;

use axum::response::IntoResponse;
use common::{PipeConfig, PipeConfigs};
use sqlx::Row;
use uuid::Uuid;

use crate::{error, Clients, Database, Workspace};

/// App contains the interactions between the application and the database.
#[derive(Debug)]
pub struct App {
    /// FIXME: Lots of controllers reach right through App to the DB and should be refactored to not do that. The end result would be this not being public.
    pub database: Database,
}

impl App {
    pub async fn new(db_path: impl AsRef<str>) -> anyhow::Result<Self> {
        let db_path: &str = db_path.as_ref();

        let database = Database::new(db_path).await?;
        Ok(Self { database })
    }

    pub async fn delete_config(&self, id: u64, user_id: &str) -> Result<(), error::Error> {
        self.database.delete_config(id, user_id).await?;
        Ok(())
    }

    pub fn validate_configs(&self, new_configs: &PipeConfigs) -> Result<(), error::Error> {
        for config in new_configs.configs.iter() {
            let pipe: Vec<serde_json::Value> = serde_json::from_value(config.pipe.clone())?;
            for p in pipe {
                // all sections need a name because we use that to identify which type of section to construct
                let name = p
                    .get("name")
                    .ok_or(error::Error::Str("section is missing 'name' field"))?;
                if name != "mycelial_server_source" && name != "mycelial_server_destination" {
                    let _client = p
                        .get("client")
                        .ok_or(error::Error::Str("section is missing 'client' field"))?;
                }
                // Should we try to construct the section here to make sure it's valid? Can we?
            }
        }
        Ok(())
    }

    /// Set pipe configs
    pub async fn set_configs(
        &self,
        new_configs: &PipeConfigs,
        user_id: &str,
    ) -> Result<Vec<u64>, error::Error> {
        let mut inserted_ids = Vec::new();
        for config in new_configs.configs.iter() {
            let id = self
                .database
                .insert_config(
                    &config.pipe,
                    config.workspace_id.try_into().unwrap(),
                    user_id,
                )
                .await?;
            inserted_ids.push(id);
        }
        Ok(inserted_ids)
    }

    pub async fn update_configs(
        &self,
        configs: PipeConfigs,
        user_id: &str,
    ) -> Result<(), error::Error> {
        for config in configs.configs {
            self.database
                .update_config(config.id, &config.pipe, user_id)
                .await?
        }
        Ok(())
    }

    pub async fn update_config(
        &self,
        config: PipeConfig,
        user_id: &str,
    ) -> Result<PipeConfig, error::Error> {
        self.database
            .update_config(config.id, &config.pipe, user_id)
            .await?;
        Ok(config)
    }

    pub async fn get_config(&self, id: u64, user_id: &str) -> Result<PipeConfig, error::Error> {
        self.database.get_config(id, user_id).await
    }

    pub async fn get_clients(&self, user_id: &str) -> Result<Clients, error::Error> {
        self.database.get_clients(user_id).await
    }

    pub async fn get_workspaces(&self, user_id: &str) -> Result<Vec<Workspace>, error::Error> {
        self.database.get_workspaces(user_id).await
    }

    pub async fn create_workspace(
        &self,
        workspace: Workspace,
        user_id: &str,
    ) -> Result<Workspace, error::Error> {
        self.database.create_workspace(workspace, user_id).await
    }

    pub async fn get_workspace(&self, id: u64, user_id: &str) -> Result<Workspace, error::Error> {
        self.database.get_workspace(id, user_id).await
    }

    pub async fn update_workspace(
        &self,
        workspace: Workspace,
        user_id: &str,
    ) -> Result<Workspace, error::Error> {
        self.database.update_workspace(workspace, user_id).await
    }

    pub async fn delete_workspace(&self, id: u64, user_id: &str) -> Result<(), error::Error> {
        self.database.delete_workspace(id, user_id).await
    }

    pub async fn get_configs(&self, user_id: &str) -> Result<PipeConfigs, error::Error> {
        self.database.get_configs(user_id).await
    }

    pub async fn get_user_id_for_daemon_token(&self, token: &str) -> Result<String, error::Error> {
        let mut connection = self.database.get_connection().await;
        let user_id: String =
            sqlx::query("SELECT user_id FROM user_daemon_tokens WHERE daemon_token = $1")
                .bind(token)
                .fetch_one(&mut *connection)
                .await
                .map(|row| row.get(0))?;
        Ok(user_id)
    }

    pub async fn get_user_daemon_token(
        &self,
        user_id: &str,
    ) -> Result<impl IntoResponse, error::Error> {
        let mut connection = self.database.get_connection().await;
        let daemon_token =
            sqlx::query("SELECT daemon_token FROM user_daemon_tokens WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&mut *connection)
                .await?
                .get::<String, _>(0);
        Ok(daemon_token)
    }

    pub async fn rotate_user_daemon_token(
        &self,
        user_id: &str,
    ) -> Result<impl IntoResponse, error::Error> {
        // create a new token
        let token = Uuid::new_v4().to_string();
        let mut connection = self.database.get_connection().await;
        // todo: Should this schema have "deleted_at" and then we only insert rows?
        sqlx::query(
            "INSERT INTO user_daemon_tokens (user_id, daemon_token) VALUES ($1, $2) ON CONFLICT (user_id) DO UPDATE SET daemon_token = $2",
        )
        .bind(user_id)
        .bind(token.clone())
        .execute(&mut *connection)
        .await?;
        Ok(token)
    }

    // should probably move this to db fn
    pub async fn validate_client_id_and_secret(
        &self,
        client_id: &str,
        secret: &str,
    ) -> Result<String, error::Error> {
        let mut connection = self.database.get_connection().await;
        let (user_id, client_secret_hash): (String, String) = sqlx::query(
            "SELECT user_id, client_secret_hash FROM clients WHERE unique_client_id = $1",
        )
        .bind(client_id)
        .fetch_one(&mut *connection)
        .await
        .map(|row| (row.get(0), row.get(1)))?;
        bcrypt::verify(secret, client_secret_hash.as_str())
            .map_err(|_| error::Error::Str("bcrypt error"))
            .and_then(|v| {
                if v {
                    Ok(user_id)
                } else {
                    Err(error::Error::Str("invalid client secret"))
                }
            })
    }
}
