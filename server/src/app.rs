use common::{PipeConfig, PipeConfigs};
use uuid::Uuid;
use crate::Result;

use crate::{
    model::{Clients, Workspace},
    db_pool::{Db, DbTrait}
};

// FIXME: squash DB into App?
/// App contains the interactions between the application and the database.
pub struct App {
    /// FIXME: Lots of controllers reach right through App to the DB and should be refactored to not do that. The end result would be this not being public.
    pub db: Box<dyn DbTrait>
}

impl App {
    pub async fn new(url: &str) -> Result<Self> {
        Ok(Self { db: crate::db_pool::new(url).await? })
    }

    pub async fn delete_config(&self, id: u64, user_id: &str) -> Result<()> {
        self.db.delete_config(id as i64, user_id).await?;
        Ok(())
    }

    pub fn validate_configs(&self, new_configs: &PipeConfigs) -> Result<()> {
        for config in new_configs.configs.iter() {
            let pipe: Vec<serde_json::Value> = serde_json::from_value(config.pipe.clone())?;
            for p in pipe {
                // all sections need a name because we use that to identify which type of section to construct
                let name = p
                    .get("name")
                    .ok_or(anyhow::anyhow!("section is missing 'name' field"))?;
                if name != "mycelial_server_source" && name != "mycelial_server_destination" {
                    let _client = p
                        .get("client")
                        .ok_or(anyhow::anyhow!("section is missing 'client' field"))?;
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
    ) -> Result<Vec<i64>> {
        // FIXME: this should be done in single transaction
        let mut inserted_ids = Vec::new();
        for config in new_configs.configs.iter() {
            let id = self.db
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
    ) -> Result<()> {
        for config in configs.configs {
            self.db
                .update_config(config.id as i64, &config.pipe, user_id)
                .await?
        }
        Ok(())
    }

    pub async fn update_config(
        &self,
        config: PipeConfig,
        user_id: &str,
    ) -> Result<PipeConfig> {
        self.db
            .update_config(config.id as i64, &config.pipe, user_id)
            .await?;
        Ok(config)
    }

    pub async fn get_config(&self, id: i64, user_id: &str) -> Result<PipeConfig> {
        Ok(self.db.get_config(id as i64, user_id).await?)
    }

    pub async fn get_clients(&self, user_id: &str) -> Result<Clients> {
        Ok(self.db.get_clients(user_id).await?)
    }

    pub async fn get_workspaces(&self, user_id: &str) -> Result<Vec<Workspace>> {
        Ok(self.db.get_workspaces(user_id).await?)
    }

    pub async fn create_workspace(
        &self,
        workspace: Workspace,
        user_id: &str,
    ) -> Result<Workspace> {
        Ok(self.db.create_workspace(workspace, user_id).await?)
    }

    pub async fn get_workspace(&self, id: i32, user_id: &str) -> Result<Option<Workspace>> {
        Ok(self.db.get_workspace(id, user_id).await?)
    }

    pub async fn update_workspace(
        &self,
        workspace: Workspace,
        user_id: &str,
    ) -> Result<Workspace> {
        Ok(self.db.update_workspace(workspace, user_id).await?)
    }

    pub async fn delete_workspace(&self, id: i32, user_id: &str) -> Result<()> {
        Ok(self.db.delete_workspace(id, user_id).await?)
    }

    pub async fn get_configs(&self, user_id: &str) -> Result<PipeConfigs> {
        let configs = self.db.get_configs(user_id).await?;
        Ok(PipeConfigs{ configs } )
    }

    pub async fn get_user_id_for_daemon_token(&self, token: &str) -> Result<Option<String>> {
        Ok(self.db.get_user_id_for_daemon_token(token).await?)
    }

    pub async fn get_user_daemon_token(&self, user_id: &str) -> Result<Option<String>> {
        Ok(self.db.get_user_daemon_token(user_id).await?)
    }

    pub async fn rotate_user_daemon_token(&self, user_id: &str) -> Result<String> {
        // create a new token
        let token = Uuid::new_v4().to_string();
        self.db
            .rotate_user_daemon_token(user_id, &token)
            .await?;
        Ok(token)
    }

    // should probably move this to db fn
    // FIXME: bcrypt is very expensive hash to check
    // usual flow is to exchange user/password into some sort of temporary token (e.g. session
    // cookie)
    // currently daemon is not able to make more than 4 rps per core against server
    // current approach is a bottleneck
    pub async fn validate_client_id_and_secret(&self, client_id: &str, secret: &str) -> Result<String> {
        let (user_id, client_secret_hash) =
        match self.db.get_user_id_and_secret_hash(client_id).await? {
            Some((user_id, client_secret_hash)) => (user_id, client_secret_hash),
            None => Err(anyhow::anyhow!("client_id {client_id} not found"))?,
        };
        match bcrypt::verify(secret, client_secret_hash.as_str())? {
            true => Ok(user_id),
            false => Err(anyhow::anyhow!("auth failed"))?
        }
    }
}
