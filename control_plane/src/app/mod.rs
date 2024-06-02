pub mod db;
pub mod migration;
pub mod model;

use self::model::Workspace;
use anyhow::Result;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct App {
    db: Arc<dyn db::DbTrait>,
}

impl App {
    pub async fn new(connection_string: &str) -> Result<Self> {
        Ok(Self {
            db: Arc::from(db::new(connection_string).await?),
        })
    }

    pub async fn init(&self) -> Result<()> {
        self.db.migrate().await
    }

    // workspaces api
    pub async fn create_workspace(&self, workspace: &Workspace) -> Result<()> {
        Ok(self.db.create_workspace(workspace).await?)
    }

    pub async fn read_workspaces(&self) -> Result<Vec<Workspace>> {
        Ok(self.db.read_workspaces().await?)
    }

    pub async fn update_workspace() {}

    pub async fn delete_workspace(&self, name: &str) -> Result<()> {
        Ok(self.db.delete_workspace(name).await?)
    }
}
