pub mod db;
pub mod model;
pub mod migration;

use anyhow::Result;

pub(crate) struct App {
    db: Box<dyn db::DbTrait>
}

impl App {
    pub async fn new(connection_string: &str) -> Result<Self> {
        Ok(Self {
            db: db::new(connection_string).await?
        })
    }

    // workspaces api
    pub async fn create_workspace() {
    }
    
    pub async fn read_workspaces() {
        
    }
    
    pub async fn update_workspace() {
        
    }
    
    pub async fn delete_workspace() {

    }
}