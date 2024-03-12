use std::sync::Arc;

use axum::{
    extract::State,
    headers::{authorization::Basic, Authorization},
    http::Request,
    middleware::Next,
    response::{IntoResponse, Response},
    Extension, Json, TypedHeader,
};
use reqwest::{header, StatusCode};
use serde::Deserialize;

use crate::{error, App, UserID};

pub async fn get_pipe_configs(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
) -> Result<impl IntoResponse, error::Error> {
    app.get_configs(user_id.0.as_str()).await.map(Json)
}

// middleware that checks for the token in the request and associates it with a client/daemon
pub async fn daemon_auth<B>(
    State(app): State<Arc<App>>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response, impl IntoResponse> {
    match app
        .validate_client_id_and_secret(auth.username(), auth.password())
        .await
    {
        Ok(user_id) => {
            let user_id = UserID(user_id);
            req.extensions_mut().insert(user_id);
            Ok(next.run(req).await)
        }
        Err(_) => Err((
            [(header::WWW_AUTHENTICATE, "Basic")],
            StatusCode::UNAUTHORIZED,
        )),
    }
}

// FIXME: common crate
#[derive(Debug, Deserialize)]
pub struct Submit {
    // FIXME:
    unique_id: String,
    sources: Vec<common::Source>,
    destinations: Vec<common::Destination>,
}

// FIXME: common crate
pub async fn submit_sections(
    State(state): State<Arc<App>>,
    Extension(UserID(user_id)): Extension<UserID>,
    Json(payload): Json<Submit>,
) -> Result<impl IntoResponse, error::Error> {
    state
        .database
        .submit_sections(
            payload.unique_id.as_str(),
            user_id.as_str(),
            &serde_json::to_string(payload.sources.as_slice())?,
            &serde_json::to_string(payload.destinations.as_slice())?,
        )
        .await?;
    Ok("")
}
