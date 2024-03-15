use std::sync::Arc;
use axum::{
    body::Body, 
    extract::State,
    http::{header, StatusCode, Request},
    middleware::Next,
    response::{IntoResponse, Response},
    Extension,
    Json
};
use axum_extra::{headers::{authorization::Basic, Authorization}, TypedHeader};
use common::{ProvisionDaemonRequest, ProvisionDaemonResponse};

use uuid::Uuid;

use crate::{App, UserID};

// FIXME: redo provisioning workflow
pub async fn provision_daemon(
    State(state): State<Arc<App>>,
    Extension(UserID(user_id)): Extension<UserID>,
    Json(payload): Json<ProvisionDaemonRequest>,
) -> crate::Result<impl IntoResponse> {
    let unique_id = payload.unique_id;
    let display_name = payload.display_name;
    let unique_client_id = Uuid::new_v4().to_string();
    let client_secret = Uuid::new_v4().to_string();
    let client_secret_hash = bcrypt::hash(client_secret.clone(), 12).unwrap();

    Ok(state
        .db
        .provision_daemon(
            &unique_id,
            &user_id,
            &display_name,
            &unique_client_id,
            &client_secret_hash,
        )
        .await
        .map(|_| {
            Json(ProvisionDaemonResponse {
                client_id: unique_client_id,
                client_secret,
            })
        })?)
}

// Token based auth
//
// Token received through ui
pub async fn auth(
    State(app): State<Arc<App>>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    match app.get_user_id_for_daemon_token(auth.username()).await {
        Ok(user_id) => {
            let user_id = UserID(user_id);
            req.extensions_mut().insert(user_id);
            Ok(next.run(req).await)
        },
        Err(_) => 
            Err(([(header::WWW_AUTHENTICATE, "Basic")], StatusCode::UNAUTHORIZED))
    }
}
