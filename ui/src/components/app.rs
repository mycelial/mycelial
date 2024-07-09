use crate::{components::routing::Route, Result};
use config_registry::{ConfigMetaData, ConfigRegistry};
use dioxus::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use super::{workspace::WorkspaceUpdate, workspaces::Workspace};

#[derive(Debug)]
pub struct AppState {
    location: url::Url,
    config_registry: ConfigRegistry,
    workspace_updates: Vec<WorkspaceUpdate>,
}

#[allow(clippy::new_without_default)]
impl AppState {
    pub fn new() -> Self {
        let location: String = web_sys::window().unwrap().location().to_string().into();
        Self {
            location: location.parse().unwrap(),
            config_registry: config_registry::new().expect("failed to initialize config registry"),
            workspace_updates: Vec::new(),
        }
    }

    fn get_url(&self, paths: &[impl AsRef<str>]) -> Result<url::Url> {
        let path = paths
            .iter()
            .map(|path| path.as_ref())
            .collect::<Vec<_>>()
            .join("/");
        Ok(self.location.join(&path)?)
    }
}

// ConfigRegistry API
impl AppState {
    pub fn menu_items(&self) -> impl Iterator<Item = ConfigMetaData> + '_ {
        self.config_registry.iter_values()
    }

    pub fn build_config(&self, name: &str) -> Result<Box<dyn config::Config>> {
        self.config_registry.build_config(name)
    }
}

// Workspaces API
const WORKSPACES_API: &str = "/api/workspaces";

impl AppState {
    pub async fn create_workspace(&self, name: &str) -> Result<()> {
        let response = reqwest::Client::new()
            .post(self.get_url(&[WORKSPACES_API])?)
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

    pub async fn read_workspaces(&self) -> Result<Vec<Workspace>> {
        Ok(reqwest::get(self.get_url(&[WORKSPACES_API])?)
            .await?
            .json()
            .await?)
    }

    pub async fn remove_workspace(&self, name: &str) -> Result<()> {
        let response = reqwest::Client::new()
            .delete(self.get_url(&[WORKSPACES_API, name])?)
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

#[derive(Debug, Deserialize)]
pub struct WorkspaceState {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Deserialize)]
pub struct Node {
    pub id: Uuid,
    pub x: f64,
    pub y: f64,
    pub config: Box<dyn config::Config>,
}

#[derive(Debug, Deserialize)]
pub struct Edge {
    pub from_id: Uuid,
    pub to_id: Uuid,
}

impl AppState {
    pub async fn fetch_workspace(&self, name: &str) -> Result<WorkspaceState> {
        let response = reqwest::get(self.get_url(&[WORKSPACE_API, name])?)
            .await?
            .json::<WorkspaceState>()
            .await?;
        Ok(response)
    }

    pub fn update_workspace(&mut self, update: WorkspaceUpdate) {
        self.workspace_updates.push(update);
    }

    pub async fn publish_updates(&mut self) -> Result<()> {
        if self.workspace_updates.is_empty() {
            return Ok(());
        }
        tracing::info!("updates: {:#?}", self.workspace_updates);
        let response = reqwest::Client::new()
            .post(self.get_url(&[WORKSPACE_API])?)
            .json(self.workspace_updates.as_slice())
            .send()
            .await?;
        match response.status().is_success() {
            true => {
                self.workspace_updates.clear();
                Ok(())
            }
            false => Err(format!(
                "failed to publish updates, response code: {}",
                response.status()
            ))?,
        }
    }
}

pub fn App() -> Element {
    // top level state
    let _app_state = use_context_provider(|| Signal::new(AppState::new()));
    rsx! {
        Router::<Route> { }
    }
}
