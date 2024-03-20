use std::sync::Arc;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use crate::{Result, model::Workspace, App, UserID};

// save a name and get an id assigned. it's a place to create pipes in
pub async fn create_workspace(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    Json(workspace): Json<Workspace>,
) -> Result<Json<Workspace>> {
    Ok(Json(app.create_workspace(workspace, user_id.0.as_str()).await?))
}

// gets a list of all the workspaces, ids, names, etc. not hydrated with pipe configs
pub async fn get_workspaces(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
) -> Result<impl IntoResponse> {
    Ok(Json(app.get_workspaces(user_id.0.as_str()).await?))
}

// by id, fetches a workspaces, hydrated with the pipe configs
pub async fn get_workspace(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    axum::extract::Path(id): axum::extract::Path<i32>,
) -> Result<impl IntoResponse> {
    match app.get_workspace(id, user_id.0.as_str()).await? {
        Some(workspace) => Ok((StatusCode::OK, Json(workspace).into_response())),
        None => Ok((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "not_found"})).into_response())),
    }
}

// updates a workspace. updating what workspace a pipe is part of is done by updating the pipe config
pub async fn update_workspace(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    axum::extract::Path(id): axum::extract::Path<u64>,
    Json(mut workspace): Json<Workspace>,
) -> Result<impl IntoResponse> {
    let id: i32 = id.try_into().unwrap();
    workspace.id = id;
    Ok(Json(app.update_workspace(workspace, user_id.0.as_str()).await?))
}

// deletes a workspace by id
pub async fn delete_workspace(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    axum::extract::Path(id): axum::extract::Path<i32>,
) -> Result<impl IntoResponse> {
    Ok(app.delete_workspace(id, user_id.0.as_str()).await?)
}
