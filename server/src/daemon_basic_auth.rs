use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Extension, Json};
use common::{ProvisionClientRequest, ProvisionClientResponse};
use uuid::Uuid;

use crate::{error, App, UserID};

pub async fn provision_client(
    State(state): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    Json(payload): Json<ProvisionClientRequest>,
) -> Result<impl IntoResponse, error::Error> {
    let client_id = payload.client_config.node.unique_id;
    let unique_client_id = Uuid::new_v4().to_string();
    let client_secret = Uuid::new_v4().to_string();
    let client_secret_hash = bcrypt::hash(client_secret.clone(), 12).unwrap();

    state
        .database
        .insert_client(
            // add the user_id here.
            &client_id,
            user_id.0.as_str(),
            &payload.client_config.node.display_name,
            &payload.client_config.sources,
            &payload.client_config.destinations,
            unique_client_id.as_str(),
            client_secret_hash.as_str(),
        )
        .await
        .map(|_| {
            Json(ProvisionClientResponse {
                id: client_id.clone(),
                client_id: unique_client_id,
                client_secret,
            })
        })
}
