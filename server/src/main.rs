#![allow(unused)]
mod app;
mod daemon_auth;
mod daemon_basic_auth;
mod db_pool;
mod ingestion;
mod model;
mod pipe;
mod workspace;
//mod db;
mod migration;

use axum::{
    body::Body,
    extract::State,
    http::{header, Method, Request, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::Utc;
use clap::Parser;
use db_pool::{Db, DbTrait};
use jsonwebtoken::{decode, jwk, DecodingKey, Validation};
use rust_embed::RustEmbed;
use sea_query::{PostgresQueryBuilder, SqliteQueryBuilder};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Sqlite};

use app::App;
use std::sync::Arc;
use std::{borrow::Cow, net::SocketAddr};

pub type Result<T, E = AppError> = core::result::Result<T, E>;

#[derive(Debug)]
pub struct AppError {
    pub status_code: StatusCode,
    pub err: anyhow::Error,
}

impl<E: Into<anyhow::Error>> From<E> for AppError {
    fn from(err: E) -> Self {
        Self {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            err: err.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let mut response = self.status_code.into_response();
        response.extensions_mut().insert(Arc::new(self));
        response
    }
}

// This struct represents the claims you expect in your Auth0 token.
#[derive(Debug, Deserialize, Serialize)]
struct MyClaims {
    // Define your claims here, for example:
    sub: String, // Subject (User ID)
                 // Add other fields as needed.
}

// Token validation logic
fn validate_token(token: &str, jwks: Auth0Jwks, audience: &str) -> Result<MyClaims> {
    let decoding_key = DecodingKey::from_jwk(&jwks.keys[0])?;

    let mut validation = Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_audience(&[audience]);
    Ok(decode::<MyClaims>(token, &decoding_key, &validation).map(|data| data.claims)?)
}

// validates token and adds the user_id to the request extensions
async fn user_auth(
    Extension(jwks): Extension<Auth0Jwks>,
    Extension(audience): Extension<Auth0Audience>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let auth0_header = req
        .headers()
        .get("x-auth0-token")
        .and_then(|header| header.to_str().ok());

    if let Some(auth0_header) = auth0_header {
        let validation_result = validate_token(auth0_header, jwks, audience.0.as_str());
        if validation_result.is_ok() {
            let user_id = validation_result.unwrap().sub;
            let user_id = UserID(user_id);
            req.extensions_mut().insert(user_id);
            return Ok(next.run(req).await);
        }
    }
    Err((StatusCode::UNAUTHORIZED, "invalid token"))
}

#[derive(Parser)]
struct Cli {
    #[clap(short, long, env = "LISTEN_ADDR", default_value = "0.0.0.0:7777")]
    listen_addr: String,

    // FIXME: no longer used
    #[clap(short, long, env = "ENDPOINT_TOKEN")]
    token: String,

    #[clap(
        short,
        long,
        env = "DATABASE_URL",
        default_value = "sqlite://control_plane.db"
    )]
    database_url: String,

    #[clap(long, env = "AUTH0_AUTHORITY", default_value = "")]
    auth0_authority: String,

    #[clap(long, env = "AUTH0_AUDIENCE", default_value = "")]
    auth0_audience: String,
}

async fn get_user_daemon_token(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
) -> Result<impl IntoResponse> {
    match app.get_user_daemon_token(user_id.0.as_str()).await? {
        Some(token) => Ok((StatusCode::OK, Cow::Owned(token))),
        None => Ok((StatusCode::NOT_FOUND, Cow::Borrowed(""))),
    }
}

async fn rotate_user_daemon_token(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
) -> Result<impl IntoResponse> {
    app.rotate_user_daemon_token(user_id.0.as_str()).await
}

async fn get_clients(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
) -> Result<impl IntoResponse> {
    Ok(Json(app.get_clients(user_id.0.as_str()).await?))
}

// log response middleware
async fn log_middleware(method: Method, uri: Uri, request: Request<Body>, next: Next) -> Response {
    let timestamp = Utc::now();
    let response = next.run(request).await;
    let request_time_ms = Utc::now()
        .signed_duration_since(timestamp)
        .num_milliseconds();

    match response.extensions().get::<Arc<AppError>>() {
        Some(error) => tracing::error!(
            request_time_ms = request_time_ms,
            method = method.as_str(),
            status_code = response.status().as_u16(),
            path = uri.path(),
            error = ?error
        ),
        None => tracing::info!(
            request_time_ms = request_time_ms,
            method = method.as_str(),
            status_code = response.status().as_u16(),
            path = uri.path(),
        ),
    };
    response
}

#[derive(RustEmbed)]
#[folder = "../console/out/"]
pub struct Assets;

async fn assets(uri: Uri) -> impl IntoResponse {
    let path = match uri.path() {
        "/" => "index.html",
        p => p,
    }
    .trim_start_matches('/');
    match Assets::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], file.data).into_response()
        }
        None => {
            // FIXME:
            // fallback to index page
            let index = Assets::get("index.html").unwrap();
            let mime = mime_guess::from_path("index.html").first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], index.data).into_response()
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserID(String);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Auth0Jwks {
    keys: Vec<jwk::Jwk>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Auth0Audience(String);

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::try_parse()?;
    let db = db_pool::new(cli.database_url.as_str()).await?;
    db.migrate().await?;
    let app = Arc::new(App::new(&cli.database_url).await?);

    let mut jwks = Auth0Jwks { keys: Vec::new() };
    let audience = Auth0Audience(cli.auth0_audience);
    if cfg!(feature = "require_auth") {
        // FIXME: cache locally
        jwks = reqwest::get(format!("{}/.well-known/jwks.json", cli.auth0_authority))
            .await?
            .json::<Auth0Jwks>()
            .await?;
    }

    let mut protected_api = Router::new()
        .route("/api/pipe", post(pipe::post_config).put(pipe::put_configs))
        .route(
            "/api/pipe/:id",
            get(pipe::get_config)
                .delete(pipe::delete_config)
                .put(pipe::put_config),
        )
        .route(
            "/api/workspaces/:id",
            get(workspace::get_workspace)
                .put(workspace::update_workspace)
                .delete(workspace::delete_workspace),
        )
        .route(
            "/api/workspaces",
            get(workspace::get_workspaces).post(workspace::create_workspace),
        )
        .route(
            "/api/daemon_token",
            post(rotate_user_daemon_token).get(get_user_daemon_token),
        )
        .route("/api/clients", get(get_clients));
    // check to see if auth feature is turned on.
    if cfg!(feature = "require_auth") {
        protected_api = protected_api.layer(middleware::from_fn(user_auth));
    } else {
        // add a dummy user_id to the request extensions, so the Extension<UserID> extractor doesn't fail
        let u = UserID("".to_string());
        protected_api = protected_api.layer(Extension(u))
    }

    let daemon_basic_api = Router::new()
        .route(
            "/api/daemon/provision",
            post(daemon_basic_auth::provision_daemon),
        )
        .layer(middleware::from_fn_with_state(
            app.clone(),
            daemon_basic_auth::auth,
        ));

    // daemon uses its client_id and client_secret to auth, regardless of whether user auth is turned on
    let daemon_protected_api = Router::new()
        .route("/api/pipe", get(daemon_auth::get_pipe_configs))
        .route(
            "/api/daemon/submit_sections",
            post(daemon_auth::submit_sections),
        )
        .layer(middleware::from_fn_with_state(
            app.clone(),
            daemon_auth::auth,
        ));

    // ingestion api is "security by obscurity" for now, and relies on the topic being secret
    let ingestion_api = Router::new()
        .route("/ingestion/:topic", post(ingestion::ingestion))
        .route("/ingestion/:topic/:offset", get(ingestion::get_message));

    // FIXME: consistent endpoint namings
    let api = Router::new()
        .merge(protected_api)
        .merge(daemon_basic_api)
        .merge(daemon_protected_api)
        .merge(ingestion_api)
        .layer(Extension(jwks))
        .layer(Extension(audience))
        .with_state(app);

    let assets = Router::new().fallback(assets);

    let router = Router::new()
        .merge(api)
        .merge(assets)
        .layer(middleware::from_fn(log_middleware));

    let addr: SocketAddr = cli.listen_addr.as_str().parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}
