pub mod db;
pub mod migration;
pub mod tables;

use anyhow::Result;
use std::sync::Arc;
use db::{Workspace, Graph};

#[derive(Clone)]
pub(crate) struct App {
    db: Arc<dyn db::DbTrait>,
}

impl App {
    pub async fn new(database_url: &str) -> Result<Self> {
        Ok(Self {
            db: Arc::from(db::new(database_url).await?),
        })
    }

    pub async fn init(&self) -> Result<()> {
        self.db.migrate().await
    }

    // workspaces api
    pub async fn create_workspace(&self, workspace: &Workspace) -> Result<()> {
        self.db.create_workspace(workspace).await
    }

    pub async fn read_workspaces(&self) -> Result<Vec<Workspace>> {
        self.db.read_workspaces().await
    }

    pub async fn delete_workspace(&self, name: &str) -> Result<()> {
        self.db.delete_workspace(name).await
    }
    
    // workspace api
    pub async fn get_graph(&self, name: &str) -> Result<Graph> {
        self.db.get_graph(name).await
    }
}
