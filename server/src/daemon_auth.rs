use std::sync::Arc;
use axum::{
    body::Body,
    extract::State,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Extension,
    Json
};
use axum_extra::{headers::{authorization::Basic, Authorization}, TypedHeader};
use serde::Deserialize;

use crate::{App, UserID};

pub async fn get_pipe_configs(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
) -> crate::Result<impl IntoResponse> {
    Ok(Json(app.get_configs(user_id.0.as_str()).await?))
}

// check client_id/client_secret from daemon, associate with user_id
pub async fn auth(
    State(app): State<Arc<App>>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    mut req: Request<Body>,
    next: Next,
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
) -> crate::Result<impl IntoResponse> {
    state
        .db
        .submit_sections(
            payload.unique_id.as_str(),
            user_id.as_str(),
            &serde_json::to_value(payload.sources)?,
            &serde_json::to_value(payload.destinations)?,
        )
        .await?;
    Ok("")
}
