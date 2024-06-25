use std::collections::VecDeque;

use crate::{
    components::routing::Route,
    config_registry::{ConfigMetaData, ConfigRegistry},
    model, Result,
};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct AppState {
    location: url::Url,
    config_registry: ConfigRegistry,
    workspace_state: VecDeque<Vec<WorkspaceOperation>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let location: String = web_sys::window().unwrap().location().to_string().into();
        Self {
            location: location.parse().unwrap(),
            config_registry: ConfigRegistry::new(),
            workspace_state: VecDeque::new(),
        }
    }

    fn get_url(&self, path: impl AsRef<str>) -> Result<url::Url> {
        Ok(self.location.join(path.as_ref())?)
    }
}

// ConfigRegistry API
impl AppState {
    pub fn menu_items(&self) -> impl Iterator<Item = ConfigMetaData> + '_ {
        self.config_registry.menu_items()
    }

    pub fn build_config(&self, name: &str) -> Box<dyn config::Config> {
        // FIXME: properly deal with missing config constructors
        self.config_registry
            .build_config(name)
            .expect("no config constructor found")
    }
}

// Workspaces API
const WORKSPACES_API: &str = "/api/workspaces";

impl AppState {
    pub async fn create_workspace(&self, name: &str) -> Result<()> {
        let response = reqwest::Client::new()
            .post(self.get_url(WORKSPACES_API)?)
            .json(&serde_json::json!({"name": name}))
            .send()
            .await?;
        match response.status().is_success() {
            true => Ok(()),
            false => Err(format!(
                "failed to create new workspace {name}, server returned {} status code",
                response.status()
            ))?,
        }
    }

    pub async fn read_workspaces(&self) -> Result<Vec<model::Workspace>> {
        Ok(reqwest::get(self.get_url(WORKSPACES_API)?)
            .await?
            .json()
            .await?)
    }

    pub async fn remove_workspace(&self, name: &str) -> Result<()> {
        let response = reqwest::Client::new()
            .delete(self.get_url(format!("{WORKSPACES_API}/{name}"))?)
            .send()
            .await?;
        match response.status().is_success() {
            true => Ok(()),
            false => Err(format!(
                "failed to delete workspace '{name}' response code: {}",
                response.status()
            ))?,
        }
    }
}

// Workspace API
const WORKSPACE_API: &str = "/api/workspace";

#[derive(Debug)]
pub struct WorkspaceRequestBuilder<'a> {
    app: &'a AppState,
    operations: Vec<WorkspaceOperation>,
}

impl<'a> WorkspaceRequestBuilder<'a> {
    pub fn new(app: &'a AppState) -> Self {
        Self {
            app,
            operations: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WorkspaceOperation {
    AddNode {},
    UpdateNode {},
    RemoveNode {},
    AddEdge {},
    RemoveEdge {},
}

impl AppState {
    pub fn add_node(&self) -> WorkspaceRequestBuilder<'_> {
        WorkspaceRequestBuilder::new(self)
    }

    pub fn update_node(&self) -> WorkspaceRequestBuilder<'_> {
        WorkspaceRequestBuilder::new(self)
    }

    pub fn remove_node(&self) -> WorkspaceRequestBuilder<'_> {
        WorkspaceRequestBuilder::new(self)
    }

    pub fn add_edge(&self) -> WorkspaceRequestBuilder<'_> {
        WorkspaceRequestBuilder::new(self)
    }

    pub fn remove_edge(&self) -> WorkspaceRequestBuilder<'_> {
        WorkspaceRequestBuilder::new(self)
    }
}

pub fn App() -> Element {
    // top level state
    let _app_state = use_context_provider(|| Signal::new(AppState::new()));
    rsx! {
        Router::<Route> { }
    }
}
