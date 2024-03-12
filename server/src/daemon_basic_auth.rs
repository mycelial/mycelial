use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Extension, Json};
use common::{ProvisionDaemonRequest, ProvisionDaemonResponse};

use uuid::Uuid;

use crate::{error, App, UserID};

// FIXME: redo provisioning workflow
pub async fn provision_daemon(
    State(state): State<Arc<App>>,
    Extension(UserID(user_id)): Extension<UserID>,
    Json(payload): Json<ProvisionDaemonRequest>,
) -> Result<impl IntoResponse, error::Error> {
    let unique_id = payload.unique_id;
    let display_name = payload.display_name;
    let unique_client_id = Uuid::new_v4().to_string();
    let client_secret = Uuid::new_v4().to_string();
    let client_secret_hash = bcrypt::hash(client_secret.clone(), 12).unwrap();

    state
        .database
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
        })
}
