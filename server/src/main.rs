//! Mycelial server
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, SqliteConnection};
use std::{collections::HashMap, str::FromStr, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;
use tower_http::cors::{Any, CorsLayer};


mod error;

use arrow::ipc::reader::StreamReader;
use axum::{
    extract::{BodyStream, State},
    headers::{authorization::Basic, Authorization},
    http::StatusCode,
    http::{Method, Request, Uri, HeaderValue},
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

#[derive(Serialize, Deserialize)]
struct Configs {
    configs: Vec<RawConfig>,
}

#[derive(Serialize, Deserialize)]
struct RawConfig {
    id: u64,
    raw_config: String,
}

async fn get_pipe_configs(State(app): State<Arc<App>>) -> Json<Configs> {
    Json(app.get_configs().await)
}

async fn post_pipe_config(State(app): State<Arc<App>>, Json(configs): Json<Configs>) -> impl IntoResponse {
    app.set_configs(configs).await;
    Json("ok")
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

    let result = state.database.insert_client(&client_id).await;
    match result {
        Ok(_) => {
            let response = ProvisionClientResponse { id: client_id };

            (StatusCode::OK, Ok(Json(response)))
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Err(e))
        }
    }
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

    // todo: It'd be smarter/safer to store the salted & hashed token in the database
    let token = Uuid::new_v4().to_string();

    let result = state.database.insert_token(&client_id, &token).await;
    match result {
        Ok(_) => {
            let response = IssueTokenResponse {
                id: token,
                client_id,
            };
            (StatusCode::OK, Ok(Json(response)))
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Err(e))
        }
    }
}

#[derive(Debug)]
#[allow(unused)]
pub struct Database {
    connection: Arc<Mutex<SqliteConnection>>,
    database_path: String,
}

impl Database {
    pub async fn new(database_dir: &str) -> Result<Self, error::Error> {
        let database_path = std::path::Path::new(database_dir)
            .join("mycelial.db")
            .to_string_lossy()
            .to_string();
        let database_url = format!("sqlite://{database_path}");
        let mut connection = SqliteConnectOptions::from_str(database_url.as_str())?
            .create_if_missing(true)
            .connect()
            .await?;
        sqlx::migrate!().run(&mut connection).await?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            database_path,
        })
    }

    pub async fn insert_client(&self, client_id: &str) -> Result<(), error::Error> {
        let mut connection = self.connection.lock().await;
        let _ = sqlx::query("INSERT INTO clients (id) VALUES (?)")
            .bind(client_id)
            .execute(&mut *connection)
            .await?;
        Ok(())
    }

    pub async fn insert_token(&self, client_id: &str, token: &str) -> Result<(), error::Error> {
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
    // FIXME: this is temporary in-memory state
    configs: Mutex<HashMap<u64, String>>,
}

impl App {
    /// Set pipe configs
    async fn set_configs(&self, new_configs: Configs) {
        let mut configs = self.configs.lock().await;
        configs.clear();
        new_configs.configs.into_iter().for_each(|config| {
            configs.insert(config.id, config.raw_config);
        });
    }

    /// Return pipe configs
    async fn get_configs(&self) -> Configs {
        let configs = self.configs.lock().await;
        Configs {
            configs: configs
                .iter()
                .map(|(&id, raw_config)| RawConfig {
                    id,
                    raw_config: raw_config.clone(),
                })
                .collect(),
        }
    }
}

impl App {
    pub async fn new(database_dir: impl AsRef<str>) -> anyhow::Result<Self> {
        tokio::fs::create_dir_all(database_dir.as_ref()).await?;
        let database = Database::new(database_dir.as_ref()).await?;
        Ok(Self {
            database,
            // FIXME: temporary in-mem storage, should be persisted in DB
            configs: Mutex::new(HashMap::new()),
        })
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = CLI::try_parse()?;
    let app = App::new("").await?;
    let state = Arc::new(app);
    // FIXME: consistent endpoint namings
    let router = Router::new()
        .route("/ingestion", post(ingestion))
        .route(
            "/pipe/configs",
            get(get_pipe_configs).post(post_pipe_config),
        )
        .route("/api/client", post(provision_client))
        .route("/api/tokens", post(issue_token))
        .layer(middleware::from_fn(log))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers(Any)
                .allow_methods([Method::GET, Method::POST]),
        )
        .with_state(state);

    let addr: SocketAddr = cli.listen_addr.as_str().parse()?;
    Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;
    Ok(())
}
