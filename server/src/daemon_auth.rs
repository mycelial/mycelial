use std::sync::Arc;

use crate::BASE64;
use axum::{
    extract::State,
    http::{self, Request},
    middleware::Next,
    response::{IntoResponse, Response},
    Extension, Json,
};
use base64::Engine;
use reqwest::{header, StatusCode};

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
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response, impl IntoResponse> {
    let auth_header = req
        .headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let decoded = auth_header
        .and_then(|header| header.strip_prefix("Basic "))
        .and_then(|token| BASE64.decode(token).ok())
        .and_then(|token| String::from_utf8(token).ok())
        .and_then(|token| {
            let parts: Vec<&str> = token.splitn(2, ':').collect();
            match parts.as_slice() {
                [client_id, client_secret] => {
                    Some((client_id.to_string(), client_secret.to_string()))
                }
                _ => None,
            }
        });

    if let Some((client_id, client_secret)) = decoded {
        let user_id = app
            .validate_client_id_and_secret(client_id.as_str(), client_secret.as_str())
            .await;
        let user_id = match user_id {
            Ok(user_id) => user_id,
            Err(_) => {
                let response = (
                    [(header::WWW_AUTHENTICATE, "Basic")],
                    StatusCode::UNAUTHORIZED,
                );
                return Err(response);
            }
        };
        let user_id = UserID(user_id);
        req.extensions_mut().insert(user_id);
        return Ok(next.run(req).await);
    }
    let response = (
        [(header::WWW_AUTHENTICATE, "Basic")],
        StatusCode::UNAUTHORIZED,
    );
    Err(response)
}
