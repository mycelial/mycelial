use std::{
    fmt::Display,
    future::{pending},
    pin::Pin,
};

use crate::components::routing::Route;
use config::StdError;
use config_registry::{ConfigMetaData, ConfigRegistry};
use dioxus::prelude::*;
use futures::{Future, FutureExt, StreamExt};
use gloo_timers::future::TimeoutFuture;
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
struct AppBackgroundCoroutine {}

impl AppBackgroundCoroutine {
    async fn enter_loop(mut rx: UnboundedReceiver<WorkspaceUpdate>) {
        let updates = &mut Vec::new();
        loop {
            let future: Pin<Box<dyn Future<Output = ()>>> = match updates.is_empty() {
                false => Box::pin(Box::pin(async { TimeoutFuture::new(100).await })),
                true => Box::pin(pending::<()>()),
            };
            futures::select! {
                msg = rx.next() => {
                    updates.push(msg.unwrap());
                },
                _ = future.fuse() => {
                    loop {
                        let response = reqwest::Client::new()
                            .post(get_url(&[WORKSPACE_API]).unwrap())
                            .json(updates.as_slice())
                            .send()
                            .await;
                        match response {
                            Ok(response) if response.status().is_success() => {
                                updates.clear();
                                break
                            },
                            Ok(response) => tracing::error!("{}", AppError::from_status_code(response.status())),
                            Err(e) => tracing::error!("Failed to perform update request: {}", e),
                        }
                        TimeoutFuture::new(1_000).await;
                    }
                }
            }
        }
    }
}
fn get_url(paths: &[impl AsRef<str>]) -> Result<url::Url> {
    let location: String = web_sys::window().unwrap().location().to_string().into();
    let location: url::Url = location.parse()?;
    let path = paths
        .iter()
        .map(|path| path.as_ref())
        .collect::<Vec<_>>()
        .join("/");
    Ok(location.join(&path)?)
}

pub struct AppState {
    config_registry: ConfigRegistry,
    coroutine_handle: Coroutine<WorkspaceUpdate>,
}

#[allow(clippy::new_without_default)]
impl AppState {
    pub fn new(coroutine_handle: Coroutine<WorkspaceUpdate>) -> Self {
        Self {
            config_registry: config_registry::new().expect("failed to initialize config registry"),
            coroutine_handle,
        }
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
            .post(get_url(&[WORKSPACES_API])?)
            .json(&serde_json::json!({"name": name}))
            .send()
            .await?;
        match response.status() {
            status_code if status_code.is_success() => Ok(()),
            status_code => Err(AppError::from_status_code(status_code)),
        }
    }

    pub async fn read_workspaces(&self) -> Result<Vec<Workspace>> {
        let response = reqwest::get(get_url(&[WORKSPACES_API])?).await?;
        match response.status() {
            StatusCode::OK => Ok(response.json().await?),
            status_code => Err(AppError::from_status_code(status_code))?,
        }
    }

    pub async fn remove_workspace(&self, name: &str) -> Result<()> {
        let response = reqwest::Client::new()
            .delete(get_url(&[WORKSPACES_API, name])?)
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
        let response = reqwest::get(get_url(&[WORKSPACE_API, name])?).await?;
        match response.status() {
            status_code if status_code.is_success() => {
                Ok(response.json::<WorkspaceState>().await?)
            }
            status_code => Err(AppError::from_status_code(status_code)),
        }
    }

    pub fn update_workspace(&mut self, update: WorkspaceUpdate) {
        self.coroutine_handle.send(update)
    }
}

pub fn App() -> Element {
    let bg_coroutine = use_coroutine(AppBackgroundCoroutine::enter_loop);
    let _app_state = use_context_provider(move || Signal::new(AppState::new(bg_coroutine)));
    rsx! {
        Router::<Route> { }
    }
}
