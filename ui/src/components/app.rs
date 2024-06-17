use crate::{components::routing::Route, model, Result};
use dioxus::prelude::*;

#[derive(Debug)]
pub struct AppState {
    location: url::Url,
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
        }
    }

    fn get_url(&self, path: impl AsRef<str>) -> Result<url::Url> {
        let res = self.location.join(path.as_ref())?;
        tracing::info!("res: {:?}", res);
        Ok(res)
    }
}

pub fn App() -> Element {
    // top level state
    let _app_state = use_context_provider(|| Signal::new(AppState::new()));
    rsx! {
        Router::<Route> { }
    }
}
