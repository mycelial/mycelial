use std::fmt::Display;

use crate::components::routing::Route;
use config::StdError;
use config_registry::{ConfigMetaData, ConfigRegistry};
use dioxus::prelude::*;
use reqwest::StatusCode;
use serde::Deserialize;
use uuid::Uuid;

use super::{workspace::WorkspaceUpdate, workspaces::Workspace};

pub type Result<T, E = AppError> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct AppError {
    pub status_code: Option<StatusCode>,
    pub err: StdError,
}

impl AppError {
    pub fn from_status_code(status_code: StatusCode) -> Self {
        Self {
            status_code: Some(status_code),
            err: status_code.to_string().into(),
        }
    }
}

impl From<StdError> for AppError {
    fn from(value: StdError) -> Self {
        AppError {
            status_code: None,
            err: value,
        }
    }
}

impl From<reqwest::Error> for AppError {
    fn from(value: reqwest::Error) -> Self {
        Self {
            status_code: None,
            err: value.into(),
        }
    }
}

impl From<url::ParseError> for AppError {
    fn from(value: url::ParseError) -> Self {
        Self {
            status_code: None,
            err: value.into(),
        }
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

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
        Ok(self.config_registry.build_config(name)?)
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
        match response.status() {
            status_code if status_code.is_success() => Ok(()),
            status_code => Err(AppError::from_status_code(status_code)),
        }
    }

    pub async fn read_workspaces(&self) -> Result<Vec<Workspace>> {
        let response = reqwest::get(self.get_url(&[WORKSPACES_API])?).await?;
        match response.status() {
            StatusCode::OK => Ok(response.json().await?),
            status_code => Err(AppError::from_status_code(status_code))?,
        }
    }

    pub async fn remove_workspace(&self, name: &str) -> Result<()> {
        let response = reqwest::Client::new()
            .delete(self.get_url(&[WORKSPACES_API, name])?)
            .send()
            .await?;
        match response.status() {
            status_code if status_code.is_success() => Ok(()),
            status_code => Err(AppError::from_status_code(status_code)),
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
        let response = reqwest::get(self.get_url(&[WORKSPACE_API, name])?).await?;
        match response.status() {
            status_code if status_code.is_success() => {
                Ok(response.json::<WorkspaceState>().await?)
            }
            status_code => Err(AppError::from_status_code(status_code)),
        }
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
        match response.status() {
            status_code if status_code.is_success() => {
                self.workspace_updates.clear();
                Ok(())
            }
            status_code => Err(AppError::from_status_code(status_code)),
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
