//! Mycelial server
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, SqliteConnection};
use std::{str::FromStr, sync::Arc};
use tokio::sync::Mutex;

use arrow::ipc::reader::StreamReader;
use axum::{
    extract::{BodyStream, State},
    headers::{authorization::Basic, Authorization},
    http::StatusCode,
    http::{Method, Request, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router, Server, TypedHeader,
};
use chrono::Utc;
use clap::Parser;
use futures::StreamExt;
use serde_json::json;

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use sqlx::migrate::MigrateError;

// TODO: figure out this error stuff, I just copied and pasted this for now.
#[derive(Debug)]
pub enum Error {
    // unauthorized request to hub endpoint
    HubAuthError,

    // unauthorized request to client endpoint
    ClientAuthError,

    // status code wrap, probably not needed
    StatusCode(StatusCode),

    // sqlx migration error
    SqlxMigrationError(MigrateError),

    // sqlx error
    SqlxError(sqlx::Error),

    // core didn't respond to message
    CoreRecvError,

    // failed to send message to core
    CoreSendError,

    //
    IoError(std::io::Error),

    // axum error wrap
    AxumError(axum::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::SqlxMigrationError(e) => Some(e),
            Error::SqlxError(e) => Some(e),
            Error::IoError(e) => Some(e),
            Error::AxumError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<StatusCode> for Error {
    fn from(s: StatusCode) -> Self {
        Self::StatusCode(s)
    }
}

impl From<MigrateError> for Error {
    fn from(e: MigrateError) -> Self {
        Self::SqlxMigrationError(e)
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self::SqlxError(e)
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for Error {
    fn from(_: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::CoreRecvError
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for Error {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::CoreSendError
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

impl From<axum::Error> for Error {
    fn from(e: axum::Error) -> Self {
        Self::AxumError(e)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let mut response: Response = match self {
            Self::StatusCode(s) => s,
            Self::HubAuthError | Self::ClientAuthError => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
        .into_response();
        response.extensions_mut().insert(self);
        response
    }
}

unsafe impl Sync for Error {}
unsafe impl Send for Error {}

#[derive(Parser)]
struct CLI {
    #[clap(short, long, env = "LISTEN_ADDR", default_value = "0.0.0.0:8080")]
    listen_addr: String,
}

// FIXME: full body accumulation
async fn ingestion(mut body: BodyStream) -> impl IntoResponse {
    let mut buf = vec![];
    while let Some(chunk) = body.next().await {
        buf.append(&mut chunk.unwrap().to_vec());
    }
    let reader = StreamReader::try_new(buf.as_slice(), None).unwrap();
    for record_batch in reader {
        println!("got new record batch:\n{:?}", record_batch);
    }
    ""
}

#[derive(Serialize)]
struct Configs {
    configs: Vec<RawConfig>,
}

#[derive(Serialize)]
struct RawConfig {
    id: u64,
    raw_config: String,
}

async fn get_pipe_configs() -> Json<Configs> {
    let configs = Configs {
        configs: vec![RawConfig {
            id: 1,
            raw_config: serde_json::to_string(&json!({
                "section": [
                    {
                        "name": "sqlite",
                        "path": "/tmp/test.sqlite",
                        "query": "select * from test",
                    },
                    {
                        "name": "mycelial_net",
                        "endpoint": "http://localhost:8080/ingestion",
                        "token": "mycelial_net_token"
                    }
                ]
            }))
            .unwrap(),
        }],
    };
    Json(configs)
}

// log response middleware
async fn log<B>(
    method: Method,
    uri: Uri,
    maybe_auth: Option<TypedHeader<Authorization<Basic>>>,
    request: Request<B>,
    next: Next<B>,
) -> Response {
    let timestamp = Utc::now();
    let response = next.run(request).await;
    let request_time_ms = Utc::now()
        .signed_duration_since(timestamp)
        .num_milliseconds();

    let token = match maybe_auth.as_ref() {
        None => "",
        Some(TypedHeader(Authorization(basic))) => basic.username(),
    };
    // FIXME: do not log token
    let log = json!({
        "token": token,
        "timestamp": timestamp,
        "request_time_ms": request_time_ms,
        "method": method.as_str(),
        "status_code": response.status().as_u16(),
        "path": uri.path(),
    });
    println!("{}", log);

    response
}

#[derive(Deserialize, Serialize)]
struct ProvisionClientRequest {
    id: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct ProvisionClientResponse {
    id: String,
}

async fn provision_client(
    State(state): State<Arc<App>>,
    Json(payload): Json<ProvisionClientRequest>,
) -> impl IntoResponse {
    let client_id = payload.id;

    // todo: do something depending on result or error
    let result = state.database.insert_client(&client_id).await;
    println!("{:?}", result);

    let response = ProvisionClientResponse { id: client_id };

    let x = Json(response);
    println!("{:?}", x);
    (StatusCode::OK, x)
}

#[derive(Deserialize)]
struct IssueTokenRequest {
    client_id: String,
}

#[derive(Serialize)]
struct IssueTokenResponse {
    id: String,
    client_id: String,
}

async fn issue_token(
    State(state): State<Arc<App>>,
    Json(payload): Json<IssueTokenRequest>,
) -> impl IntoResponse {
    let client_id = payload.client_id.clone();
    // todo: generate a random token
    let token = "test123".to_string();

    // todo: do something depending on result or error
    let result = state.database.insert_token(&client_id, &token).await;

    let response = IssueTokenResponse {
        id: token,
        client_id: client_id,
    };

    (StatusCode::OK, Json(response))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = CLI::try_parse()?;
    let app = App::new("".to_string()).await?;
    let state = Arc::new(app);
    let router = Router::new()
        .route("/ingestion", post(ingestion))
        .route("/pipe/configs", get(get_pipe_configs))
        .layer(middleware::from_fn(log))
        .merge(
            Router::new()
                .route("/api/client", post(provision_client))
                .route("/api/tokens", post(issue_token))
                .with_state(state)
                .layer(middleware::from_fn(log)),
        );

    let addr: SocketAddr = cli.listen_addr.as_str().parse()?;
    Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;
    Ok(())
}

#[derive(Debug)]
#[allow(unused)]
pub struct Database {
    connection: Arc<Mutex<SqliteConnection>>,
    database_path: String,
}

impl Database {
    pub async fn new(database_dir: &str) -> Result<Self, Error> {
        let _ = SqliteConnectOptions::from_str("sqlite://:memory:")?
            .connect()
            .await?;
        let database_path = std::path::Path::new(database_dir)
            .join("mycelial.db")
            .to_string_lossy()
            .to_string();
        let database_url = format!("sqlite://{database_path}");
        let mut connection = SqliteConnectOptions::from_str(database_url.as_str())?
            .create_if_missing(true)
            .connect()
            .await?;
        // reset default endpoint to disable replicator queries
        sqlx::migrate!().run(&mut connection).await?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            database_path,
        })
    }

    pub async fn insert_client(&self, client_id: &str) -> Result<(), Error> {
        let mut connection = self.connection.lock().await;
        let _ = sqlx::query("INSERT INTO clients (id) VALUES (?)")
            .bind(client_id)
            .execute(&mut *connection)
            .await?;
        Ok(())
    }

    pub async fn insert_token(&self, client_id: &str, token: &str) -> Result<(), Error> {
        let mut connection = self.connection.lock().await;
        let _ = sqlx::query(
            "INSERT INTO tokens (client_id, id) VALUES (?,?) ON CONFLICT (id) DO NOTHING",
        )
        .bind(client_id)
        .bind(token)
        .execute(&mut *connection)
        .await?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct App {
    database: Database,
}

impl App {
    pub async fn new(database_dir: String) -> anyhow::Result<Self> {
        tokio::fs::create_dir_all(database_dir.as_str()).await?;

        let database = Database::new(database_dir.as_str()).await?;

        Ok(Self { database })
    }
}
