use std::{fmt::Display, future::pending, ops::Deref, pin::Pin, rc::Rc};

use crate::components::routing::Route;
use chrono::{DateTime, Utc};
use config::prelude::RawConfig;
use config_registry::ConfigRegistry as _ConfigRegistry;
use dioxus::prelude::*;
use futures::{Future, FutureExt, StreamExt};
use gloo_timers::future::TimeoutFuture;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{graph::GraphOperation, node_state_form::NodeState};

pub type Result<T, E = AppError> = std::result::Result<T, E>;

pub type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;

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
                            Ok(response) => {
                                tracing::error!("{}", AppError::from_status_code(response.status()));
                                break
                            },
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

#[derive(Clone, Copy)]
pub struct ControlPlaneClient {
    coroutine_handle: Coroutine<WorkspaceUpdate>,
}

// component prop requrement
impl PartialEq for ControlPlaneClient {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl ControlPlaneClient {
    pub fn new(coroutine_handle: Coroutine<WorkspaceUpdate>) -> Self {
        Self { coroutine_handle }
    }
}

// Workspaces API
const WORKSPACES_API: &str = "/api/workspaces";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub created_at: DateTime<Utc>,
}

impl ControlPlaneClient {
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

#[derive(Debug, Clone, Serialize)]
pub enum WorkspaceOperation {
    AddNode(NodeState),
    RemoveNode(Uuid),
    UpdateNodePosition { uuid: Uuid, x: f64, y: f64 },
    UpdateNodeConfig { id: Uuid, config: RawConfig },
    AddEdge { from: Uuid, to: Uuid },
    RemoveEdge { from: Uuid, to: Uuid },
    UpdatePan { x: f64, y: f64 },
}

impl From<GraphOperation<Uuid, Signal<NodeState>>> for WorkspaceOperation {
    fn from(value: GraphOperation<Uuid, Signal<NodeState>>) -> Self {
        match value {
            GraphOperation::AddNode(node) => Self::AddNode(node.read().clone()),
            GraphOperation::RemoveNode(node) => Self::RemoveNode(node.read().id),
            GraphOperation::AddEdge(from, to) => Self::AddEdge { from, to },
            GraphOperation::RemoveEdge(from, to) => Self::RemoveEdge { from, to },
        }
    }
}

impl From<GraphOperation<Uuid, Signal<NodeState>>> for Vec<WorkspaceOperation> {
    fn from(val: GraphOperation<Uuid, Signal<NodeState>>) -> Self {
        let workspace_op: WorkspaceOperation = val.into();
        vec![workspace_op]
    }
}

#[derive(Debug, Serialize)]
pub struct WorkspaceUpdate {
    name: Rc<str>,
    operations: Vec<WorkspaceOperation>,
}

impl WorkspaceUpdate {
    pub fn new(name: &Rc<str>, operations: impl Into<Vec<WorkspaceOperation>>) -> Self {
        Self {
            name: Rc::clone(name),
            operations: operations.into(),
        }
    }
}

impl ControlPlaneClient {
    pub async fn get_workspace(&self, name: &str) -> Result<WorkspaceState> {
        let response = reqwest::get(get_url(&[WORKSPACE_API, name])?).await?;
        match response.status() {
            status_code if status_code.is_success() => {
                Ok(response.json::<WorkspaceState>().await?)
            }
            status_code => Err(AppError::from_status_code(status_code)),
        }
    }

    pub fn update_workspace(&self, update: WorkspaceUpdate) {
        self.coroutine_handle.send(update)
    }
}

// Daemon && Daemon Tokens API

const DAEMON_API: &str = "/api/daemon";

#[derive(Debug, Clone, Deserialize)]
pub struct Token {
    pub id: Uuid,
    pub secret: String,
    pub issued_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct Daemon {
    pub id: Uuid,
    pub name: String,
    pub address: Option<String>,
    pub last_seen: Option<DateTime<Utc>>,
    pub joined_at: Option<DateTime<Utc>>,
    pub status: DaemonStatus,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub enum DaemonStatus {
    Online,
    Offline,
}

impl Display for DaemonStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl ControlPlaneClient {
    pub async fn get_tokens(&self) -> Result<Vec<Token>> {
        let response = reqwest::get(get_url(&[DAEMON_API, "tokens"])?).await?;
        match response.status() {
            status_code if status_code.is_success() => Ok(response.json().await?),
            status_code => Err(AppError::from_status_code(status_code)),
        }
    }

    pub async fn create_token(&self) -> Result<Token> {
        let response = reqwest::Client::new()
            .post(get_url(&[DAEMON_API, "tokens"])?)
            .send()
            .await?;
        match response.status() {
            status_code if status_code.is_success() => Ok(response.json().await?),
            status_code => Err(AppError::from_status_code(status_code)),
        }
    }

    pub async fn delete_token(&self, id: Uuid) -> Result<()> {
        let response = reqwest::Client::new()
            .delete(get_url(&[DAEMON_API, "tokens", id.to_string().as_str()])?)
            .send()
            .await?;
        match response.status() {
            status_code if status_code.is_success() => Ok(()),
            status_code => Err(AppError::from_status_code(status_code)),
        }
    }

    pub async fn get_daemons(&self) -> Result<Vec<Daemon>> {
        let response = reqwest::Client::new()
            .get(get_url(&[DAEMON_API])?)
            .send()
            .await?;
        match response.status() {
            status_code if status_code.is_success() => Ok(response.json().await?),
            status_code => Err(AppError::from_status_code(status_code)),
        }
    }

    pub async fn remove_daemon(&self, id: Uuid) -> Result<()> {
        let id: String = id.to_string();
        let response = reqwest::Client::new()
            .delete(get_url(&[DAEMON_API, "daemons", id.as_str()])?)
            .send()
            .await?;
        match response.status() {
            status_code if status_code.is_success() => Ok(response.json().await?),
            status_code => Err(AppError::from_status_code(status_code)),
        }
    }
}

#[derive(Clone)]
pub struct ConfigRegistry(Rc<_ConfigRegistry>);

impl Deref for ConfigRegistry {
    type Target = Rc<_ConfigRegistry>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq for ConfigRegistry {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

pub fn App() -> Element {
    let bg_coroutine = use_coroutine(AppBackgroundCoroutine::enter_loop);
    let _control_plane_client = use_context_provider(move || ControlPlaneClient::new(bg_coroutine));
    let _config_registry = use_context_provider(move || {
        ConfigRegistry(Rc::new(
            config_registry::new().expect("failed to initialize config registry"),
        ))
    });
    rsx! {
        Router::<Route> { }
    }
}
