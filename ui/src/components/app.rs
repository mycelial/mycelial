use std::collections::VecDeque;

use crate::{
    components::routing::Route,
    config_registry::{ConfigMetaData, ConfigRegistry},
    Result,
};
use dioxus::prelude::*;

use super::{workspace::WorkspaceOperation, workspaces::Workspace};

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

    fn get_url(&self, paths: &[impl AsRef<str>]) -> Result<url::Url> {
        let mut url = self.location.clone();
        for path in paths {
            url = url.join(path.as_ref())?
        }
        Ok(url)
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

// AppState accumulutas changes which will be released by publishing
impl AppState {
    pub async fn fetch_workspace(&self, name: &str) -> Result<()> {
        let url = self.get_url(&[WORKSPACE_API, name])?;
        Ok(())
    }

    pub async fn update_workspace(&self) -> Result<()> {
        Ok(())
    }
}

pub fn App() -> Element {
    // top level state
    let _app_state = use_context_provider(|| Signal::new(AppState::new()));
    rsx! {
        Router::<Route> { }
    }
}
