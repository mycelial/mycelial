use crate::Result;
use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Extension, Json};
use common::{PipeConfig, PipeConfigs};

use crate::{App, UserID};

pub async fn post_config(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    Json(configs): Json<PipeConfigs>,
) -> Result<impl IntoResponse> {
    tracing::trace!("Configs in: {:?}", &configs);
    app.validate_configs(&configs)?;
    let ids = app.set_configs(&configs, user_id.0.as_str()).await?;
    Ok(Json(
        ids.iter()
            .zip(configs.configs)
            .map(|(id, conf)| PipeConfig {
                id: (*id) as u64,
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
) -> Result<impl IntoResponse> {
    Ok(Json(app.get_config(id as i64, user_id.0.as_str()).await?))
}

pub async fn put_configs(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    Json(configs): Json<PipeConfigs>,
) -> Result<impl IntoResponse> {
    app.validate_configs(&configs)?;
    Ok(Json(app.update_configs(configs, user_id.0.as_str()).await?))
}

pub async fn put_config(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    axum::extract::Path(id): axum::extract::Path<u64>,
    Json(mut config): Json<PipeConfig>,
) -> Result<impl IntoResponse> {
    // FIXME: what is that?
    config.id = id;
    app
        .update_config(config, user_id.0.as_str())
        .await
        .map(Json)
}

pub async fn delete_config(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<impl IntoResponse> {
    app.delete_config(id, user_id.0.as_str()).await
}
