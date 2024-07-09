pub mod db;
pub mod migration;
pub mod tables;

use anyhow::Result;
use config::prelude::*;
use config_registry::{self, ConfigRegistry};
use db::{Graph, Workspace};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct WorkspaceUpdate {
    name: String,
    operations: Vec<WorkspaceOperation>,
}

#[derive(Debug, Deserialize)]
pub enum WorkspaceOperation {
    AddNode {
        id: Uuid,
        x: f64,
        y: f64,
        config: Box<dyn config::Config>,
    },
    UpdateNodePosition {
        uuid: Uuid,
        x: f64,
        y: f64,
    },
    RemoveNode(Uuid),
    AddEdge {
        from: Uuid,
        to: Uuid,
    },
    RemoveEdge {
        from: Uuid,
    },
}

#[derive(Clone)]
pub(crate) struct App {
    db: Arc<dyn db::DbTrait>,
    config_registry: Arc<ConfigRegistry>,
}

impl App {
    pub async fn new(database_url: &str) -> Result<Self> {
        Ok(Self {
            db: Arc::from(db::new(database_url).await?),
            config_registry: Arc::new(
                config_registry::new()
                    .map_err(|e| anyhow::anyhow!("failed to build config registry: {e}"))?,
            ),
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

    pub async fn update_workspace(&self, updates: &mut [WorkspaceUpdate]) -> Result<()> {
        // validate configs
        for update in updates.iter_mut() {
            for operation in update.operations.as_mut_slice() {
                if let WorkspaceOperation::AddNode { config, .. } = operation {
                    let config_name = config.name();
                    let mut default_config = self
                        .config_registry
                        .build_config(config_name)
                        .map_err(|e| {
                            anyhow::anyhow!("failed to build config for {config_name}: {e}")
                        })?;
                    deserialize_into_config(&**config, &mut *default_config).map_err(|e| {
                        anyhow::anyhow!("failed to deserialize config {config_name}: {e}")
                    })?;
                    // FIXME: field validation
                    // default_config.validate()

                    // swap deserilized config
                    std::mem::swap(config, &mut default_config);
                }
            }
        }
        self.db.update_workspace(updates).await
    }
}
