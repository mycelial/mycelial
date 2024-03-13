use axum::{
    extract::State,
    http::{self, header, Method, Request, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router, Server,
};
use base64::engine::{general_purpose::STANDARD as BASE64, Engine};
use chrono::NaiveDateTime;
use chrono::{DateTime, Utc};
use clap::Parser;
use common::{Destination, PipeConfig, PipeConfigs, Source};
use futures::{Stream, StreamExt};
use jsonwebtoken::{decode, jwk, DecodingKey, Validation};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};

use sqlx::{
    pool::Pool,
    postgres::{PgPoolOptions, PgRow, Postgres},
    FromRow, Row, Transaction,
};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use app::App;
mod app;
mod daemon_auth;
mod daemon_basic_auth;
pub mod error;
mod ingestion;
mod pipe;
mod workspace;

// This struct represents the claims you expect in your Auth0 token.
#[derive(Debug, Deserialize, Serialize)]
struct MyClaims {
    // Define your claims here, for example:
    sub: String, // Subject (User ID)
                 // Add other fields as needed.
}

// Token validation logic
fn validate_token(
    token: &str,
    jwks: Auth0Jwks,
    audience: &str,
) -> Result<MyClaims, jsonwebtoken::errors::Error> {
    let decoding_key = DecodingKey::from_jwk(&jwks.keys[0])?;

    let mut validation = Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_audience(&[audience]);
    decode::<MyClaims>(token, &decoding_key, &validation).map(|data| data.claims)
}

async fn validate_client_basic_auth<B>(
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
                [auth_token, _] => Some(auth_token.to_string()),
                _ => None,
            }
        });

    if let Some(auth_token) = decoded {
        let user_id = app.get_user_id_for_daemon_token(auth_token.as_str()).await;
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

// validates token and adds the user_id to the request extensions
async fn user_auth<B>(
    Extension(jwks): Extension<Auth0Jwks>,
    Extension(audience): Extension<Auth0Audience>,
    mut req: Request<B>,
    next: Next<B>,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Workspace {
    #[serde(default)]
    pub id: i32,
    #[serde(default)]
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub pipe_configs: Vec<PipeConfig>,
    pub name: String,
}

impl FromRow<'_, PgRow> for Workspace {
    fn from_row(row: &PgRow) -> sqlx::Result<Self> {
        let created_at: NaiveDateTime = row.get("created_at");
        Ok(Self {
            id: row.get("id"),
            name: row.get("name"),
            created_at: chrono::TimeZone::from_utc_datetime(&Utc, &created_at),
            pipe_configs: Vec::new(),
        })
    }
}

#[derive(Parser)]
struct Cli {
    #[clap(short, long, env = "LISTEN_ADDR", default_value = "0.0.0.0:7777")]
    listen_addr: String,

    /// Server authorization token
    #[clap(short, long, env = "ENDPOINT_TOKEN")]
    token: String,

    /// Database path
    #[clap(
        short,
        long,
        env = "DATABASE_PATH",
        default_value = "postgres://mycelial@localhost/mycelial_server_dev"
    )]
    database_path: String,

    #[clap(long, env = "AUTH0_AUTHORITY", default_value = "")]
    auth0_authority: String,

    #[clap(long, env = "AUTH0_AUDIENCE", default_value = "")]
    auth0_audience: String,
}

struct MessageStream {
    id: u64,
    origin: String,
    stream_type: String,
    stream: Pin<Box<dyn Stream<Item = Result<Vec<u8>, error::Error>> + Send>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Clients {
    clients: Vec<Client>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Client {
    id: String,
    display_name: String,
    #[serde(default)]
    sources: Vec<Source>,
    #[serde(default)]
    destinations: Vec<Destination>,
}

async fn get_user_daemon_token(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
) -> Result<impl IntoResponse, error::Error> {
    app.get_user_daemon_token(user_id.0.as_str()).await
}

async fn rotate_user_daemon_token(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
) -> Result<impl IntoResponse, error::Error> {
    app.rotate_user_daemon_token(user_id.0.as_str()).await
}

async fn get_clients(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
) -> Result<impl IntoResponse, error::Error> {
    app.get_clients(user_id.0.as_str()).await.map(Json)
}

// log response middleware
async fn log_middleware<B>(
    method: Method,
    uri: Uri,
    request: Request<B>,
    next: Next<B>,
) -> Response {
    let timestamp = Utc::now();
    let response = next.run(request).await;
    let request_time_ms = Utc::now()
        .signed_duration_since(timestamp)
        .num_milliseconds();

    let error: Option<&error::Error> = response.extensions().get();
    tracing::info!(
        request_time_ms = request_time_ms,
        method = method.as_str(),
        status_code = response.status().as_u16(),
        path = uri.path(),
        error = error.map(|e| format!("{:?}", e)),
    );
    response
}

#[derive(Debug)]
#[allow(unused)]
pub struct Database {
    connection: Arc<Pool<Postgres>>,
    database_path: String,
}

impl Database {
    async fn get_connection(&self) -> sqlx::pool::PoolConnection<Postgres> {
        self.connection.acquire().await.unwrap()
    }

    async fn new(database_path: &str) -> Result<Self, error::Error> {
        let database_url = database_path.to_string();
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await?;

        sqlx::migrate!().run(&pool).await?;
        Ok(Self {
            database_path: database_path.into(),
            connection: Arc::new(pool),
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn provision_daemon(
        &self,
        unique_id: &str,
        user_id: &str,
        display_name: &str,
        unique_client_id: &str,
        client_secret_hash: &str,
    ) -> Result<(), error::Error> {
        sqlx::query(
            "INSERT INTO clients (id, user_id, display_name, sources, destinations, unique_client_id, client_secret_hash) \
            VALUES ($1, $2, $3, '[]', '[]', $4, $5) \
            ON CONFLICT (id) DO UPDATE SET \
            display_name=excluded.display_name, \
            sources=excluded.sources, \
            destinations=excluded.destinations, \
            unique_client_id=excluded.unique_client_id, \
            client_secret_hash=excluded.client_secret_hash"
        )
            .bind(unique_id)
            .bind(user_id)
            .bind(display_name)
            .bind(unique_client_id)
            .bind(client_secret_hash)
            .execute(&*self.connection)
            .await?;
        Ok(())
    }

    async fn submit_sections(
        &self,
        unique_id: &str,
        user_id: &str,
        sources: &str,
        destinations: &str,
    ) -> Result<(), error::Error> {
        sqlx::query("UPDATE clients SET sources=$1, destinations=$2 WHERE id=$3 AND user_id=$4")
            .bind(sources)
            .bind(destinations)
            .bind(unique_id)
            .bind(user_id)
            .execute(&*self.connection)
            .await?;
        Ok(())
    }

    async fn insert_config(
        &self,
        config: &serde_json::Value,
        workspace_id: i32,
        user_id: &str,
    ) -> Result<u64, error::Error> {
        let config: String = serde_json::to_string(config)?;
        let result =
            sqlx::query("INSERT INTO pipes (raw_config, workspace_id, user_id) VALUES ($1::json, $2, $3) RETURNING id")
                .bind(config)
                .bind(workspace_id)
                .bind(user_id)
                .fetch_one(&*self.connection)
                .await?;
        let id = result.get::<i32, _>(0);
        Ok(id.try_into().unwrap())
    }

    async fn update_config(
        &self,
        id: u64,
        config: &serde_json::Value,
        user_id: &str,
    ) -> Result<(), error::Error> {
        let config: String = serde_json::to_string(config)?;
        // FIXME: unwrap
        let id: i32 = id.try_into().unwrap();
        let _ =
            sqlx::query("update pipes set raw_config = $1::json WHERE id = $2 and user_id = $3")
                .bind(config)
                .bind(id)
                .bind(user_id)
                .execute(&*self.connection)
                .await?;
        Ok(())
    }

    async fn delete_config(&self, id: u64, user_id: &str) -> Result<(), error::Error> {
        // FIXME: unwrap
        let id: i32 = id.try_into().unwrap();
        let _ = sqlx::query("DELETE FROM pipes WHERE id = $1 and user_id = $2")
            .bind(id) // fixme
            .bind(user_id)
            .execute(&*self.connection)
            .await?;
        Ok(())
    }

    async fn new_message(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        topic: &str,
        origin: &str,
        stream_type: &str,
    ) -> Result<i32, error::Error> {
        let id = sqlx::query(
            "INSERT INTO messages(topic, origin, stream_type) VALUES($1, $2, $3) RETURNING ID",
        )
        .bind(topic)
        .bind(origin)
        .bind(stream_type)
        .fetch_one(&mut **transaction)
        .await
        .map(|row| row.get::<i32, _>(0))?;
        Ok(id)
    }

    async fn store_chunk(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        message_id: i32,
        bytes: &[u8],
    ) -> Result<(), error::Error> {
        sqlx::query("INSERT INTO records (message_id, data) VALUES ($1, $2)")
            .bind(message_id)
            .bind(bytes)
            .execute(&mut **transaction)
            .await?;
        Ok(())
    }

    async fn get_message(
        &self,
        topic: &str,
        offset: u64,
    ) -> Result<Option<MessageStream>, error::Error> {
        let mut con = self.get_connection().await;

        // FIXME: unwrap
        let offset: i32 = offset.try_into().unwrap();
        let message_info = sqlx::query(
            "SELECT id, origin, stream_type FROM messages WHERE id > $1 and topic = $2 LIMIT 1",
        )
        .bind(offset)
        .bind(topic)
        .fetch_optional(&mut *con)
        .await?
        .map(|row| {
            (
                row.get::<i32, _>(0) as u64,
                row.get::<String, _>(1),
                row.get::<String, _>(2),
            )
        });

        let (id, origin, stream_type) = match message_info {
            Some((id, o, t)) => (id, o, t),
            None => return Ok(None),
        };

        // move connection into stream wrapper around sqlx's stream
        let stream = async_stream::stream! {
            let mut stream = sqlx::query("SELECT data FROM records r WHERE r.message_id = $1")
                .bind(id as i32)
                .fetch(&mut *con)
                .map(|maybe_row| {
                    maybe_row
                        .map(|row| row.get::<Vec<u8>, &str>("data"))
                        .map_err(Into::into)
                });
            while let Some(chunk) = stream.next().await {
                yield chunk;
            }
        };
        Ok(Some(MessageStream {
            id,
            origin,
            stream_type,
            stream: Box::pin(stream),
        }))
    }

    async fn get_clients(&self, user_id: &str) -> Result<Clients, error::Error> {
        // todo: should we return ui as client?
        let mut query = sqlx::query("SELECT id, display_name, sources, destinations FROM clients");
        if cfg!(feature = "require_auth") {
            query = sqlx::query(
                "SELECT id, display_name, sources, destinations FROM clients where user_id = $1",
            )
            .bind(user_id);
        }
        let rows = query.fetch_all(&*self.connection).await?;

        let mut clients: Vec<Client> = Vec::new();
        for row in rows.iter() {
            let id = row.get("id");
            let display_name = row.get("display_name");
            let sources: Option<String> = row.get("sources");
            let sources = serde_json::from_str(&sources.unwrap_or("[]".to_string()))?;
            let destinations: Option<String> = row.get("destinations");
            let destinations = serde_json::from_str(&destinations.unwrap_or("[]".to_string()))?;
            clients.push(Client {
                id,
                display_name,
                sources,
                destinations,
            });
        }
        Ok(Clients { clients })
    }

    async fn get_workspaces(&self, user_id: &str) -> Result<Vec<Workspace>, error::Error> {
        let records: Vec<Workspace> =
            sqlx::query_as(r"SELECT id, name, created_at FROM workspaces where user_id = $1")
                .bind(user_id)
                .fetch_all(&*self.connection)
                .await?;

        Ok(records)
    }

    async fn create_workspace(
        &self,
        mut workspace: Workspace,
        user_id: &str,
    ) -> Result<Workspace, error::Error> {
        let result =
            sqlx::query("INSERT INTO workspaces (name, user_id) VALUES ($1, $2) RETURNING id")
                .bind(workspace.name.clone())
                .bind(user_id)
                .fetch_one(&*self.connection)
                .await?;
        let id = result.get::<i32, _>(0);
        workspace.id = id;
        workspace.created_at = Utc::now();
        Ok(workspace)
    }

    async fn get_workspace(&self, id: u64, user_id: &str) -> Result<Workspace, error::Error> {
        let id: i32 = id.try_into().unwrap();
        let mut record: Workspace = sqlx::query_as(
            r"SELECT id, name, created_at FROM workspaces WHERE id = $1 and user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .fetch_one(&*self.connection)
        .await?;

        let pipes: Vec<PipeConfig> = sqlx::query_as(
            "SELECT id, raw_config, workspace_id from pipes where workspace_id = $1 and user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .fetch_all(&*self.connection)
        .await?;

        record.pipe_configs = pipes;

        Ok(record)
    }

    async fn update_workspace(
        &self,
        workspace: Workspace,
        user_id: &str,
    ) -> Result<Workspace, error::Error> {
        let _ = sqlx::query("UPDATE workspaces SET name = $1 where id = $2 and user_id = $3")
            .bind(workspace.name.clone())
            .bind(workspace.id)
            .bind(user_id)
            .execute(&*self.connection)
            .await?;
        Ok(workspace)
    }

    async fn delete_workspace(&self, id: u64, user_id: &str) -> Result<(), error::Error> {
        let id: i64 = id.try_into().unwrap();
        let _ = sqlx::query("DELETE FROM workspaces WHERE id = $1 and user_id = $2")
            .bind(id) // fixme
            .bind(user_id)
            .execute(&*self.connection)
            .await?;
        Ok(())
    }

    async fn get_config(&self, id: u64, user_id: &str) -> Result<PipeConfig, error::Error> {
        let id: i64 = id.try_into().unwrap();
        let pipe: PipeConfig = sqlx::query_as(
            "SELECT id, workspace_id, raw_config from pipes WHERE id = $1 and user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .fetch_one(&*self.connection)
        .await?;
        Ok(pipe)
    }

    async fn get_configs(&self, user_id: &str) -> Result<PipeConfigs, error::Error> {
        let rows: Vec<PipeConfig> =
            sqlx::query_as("SELECT id, raw_config, workspace_id from pipes where user_id = $1")
                .bind(user_id)
                .fetch_all(&*self.connection)
                .await?;

        let configs: PipeConfigs = PipeConfigs { configs: rows };
        Ok(configs)
    }

    async fn get_user_id_for_daemon_token(&self, token: &str) -> Result<String, error::Error> {
        let user_id: String =
            sqlx::query("SELECT user_id FROM user_daemon_tokens WHERE daemon_token = $1")
                .bind(token)
                .fetch_one(&*self.connection)
                .await
                .map(|row| row.get(0))?;
        Ok(user_id)
    }

    async fn get_user_daemon_token(&self, user_id: &str) -> Result<String, error::Error> {
        let daemon_token =
            sqlx::query("SELECT daemon_token FROM user_daemon_tokens WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&*self.connection)
                .await?
                .get::<String, _>(0);
        Ok(daemon_token)
    }

    async fn rotate_user_daemon_token(
        &self,
        user_id: &str,
        new_token: &str,
    ) -> Result<(), error::Error> {
        // todo: Should this schema have "deleted_at" and then we only insert rows?
        sqlx::query(
            "INSERT INTO user_daemon_tokens (user_id, daemon_token) VALUES ($1, $2) ON CONFLICT (user_id) DO UPDATE SET daemon_token = $2"
        )
        .bind(user_id)
        .bind(new_token)
        .execute(&*self.connection)
        .await?;
        Ok(())
    }

    async fn get_user_id_and_secret_hash(
        &self,
        client_id: &str,
    ) -> Result<(String, String), error::Error> {
        let (user_id, client_secret_hash): (String, String) = sqlx::query(
            "SELECT user_id, client_secret_hash FROM clients WHERE unique_client_id = $1",
        )
        .bind(client_id)
        .fetch_one(&*self.connection)
        .await
        .map(|row| (row.get(0), row.get(1)))?;
        Ok((user_id, client_secret_hash))
    }
}

#[derive(RustEmbed)]
#[folder = "../console/out/"]
pub struct Assets;

async fn assets(uri: Uri) -> Result<impl IntoResponse, StatusCode> {
    let path = match uri.path() {
        "/" => "index.html",
        p => p,
    }
    .trim_start_matches('/');
    match Assets::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Ok(([(header::CONTENT_TYPE, mime.as_ref())], file.data).into_response())
        }
        None => {
            // fallback to index page
            let index = Assets::get("index.html").unwrap();
            let mime = mime_guess::from_path("index.html").first_or_octet_stream();
            Ok(([(header::CONTENT_TYPE, mime.as_ref())], index.data).into_response())
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
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::try_parse()?;
    let app = App::new(cli.database_path).await?;
    let state = Arc::new(app);

    let mut jwks = Auth0Jwks { keys: Vec::new() };
    let audience = Auth0Audience(cli.auth0_audience);
    if cfg!(feature = "require_auth") {
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
            state.clone(),
            validate_client_basic_auth,
        ));

    // daemon uses its client_id and client_secret to auth, regardless of whether user auth is turned on
    let daemon_protected_api = Router::new()
        .route("/api/pipe", get(daemon_auth::get_pipe_configs))
        .route(
            "/api/daemon/submit_sections",
            post(daemon_auth::submit_sections),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            daemon_auth::daemon_auth,
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
        .with_state(state.clone());

    let assets = Router::new().fallback(assets);

    let router = Router::new()
        .merge(api)
        .merge(assets)
        .layer(middleware::from_fn(log_middleware));

    let addr: SocketAddr = cli.listen_addr.as_str().parse()?;
    Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;
    Ok(())
}
