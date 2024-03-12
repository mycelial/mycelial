use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Extension, Json};
use common::{PipeConfig, PipeConfigs};

use crate::{error, App, UserID};

pub async fn post_config(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    Json(configs): Json<PipeConfigs>,
) -> Result<impl IntoResponse, error::Error> {
    tracing::trace!("Configs in: {:?}", &configs);
    app.validate_configs(&configs)?;
    let ids = app.set_configs(&configs, user_id.0.as_str()).await?;
    Ok(Json(
        ids.iter()
            .zip(configs.configs)
            .map(|(id, conf)| PipeConfig {
                id: *id,
                pipe: conf.pipe,
                workspace_id: conf.workspace_id,
            })
            .collect::<Vec<PipeConfig>>(),
    )
    .into_response())
}

pub async fn get_config(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<impl IntoResponse, error::Error> {
    app.get_config(id, user_id.0.as_str()).await.map(Json)
}

pub async fn put_configs(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    Json(configs): Json<PipeConfigs>,
) -> Result<impl IntoResponse, error::Error> {
    app.validate_configs(&configs)?;
    app.update_configs(configs, user_id.0.as_str())
        .await
        .map(Json)
}

pub async fn put_config(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    axum::extract::Path(id): axum::extract::Path<u64>,
    Json(mut config): Json<PipeConfig>,
) -> Result<impl IntoResponse, error::Error> {
    config.id = id;
    app.update_config(config, user_id.0.as_str())
        .await
        .map(Json)
}

pub async fn delete_config(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<impl IntoResponse, error::Error> {
    app.delete_config(id, user_id.0.as_str()).await
}
