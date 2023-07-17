//! Mycelial server
use arrow::{ipc::reader::StreamReader, util::pretty::pretty_format_batches};
use base64::engine::{general_purpose::STANDARD as BASE64, Engine};
use axum::{
    extract::{BodyStream, State},
    headers::{authorization::Basic, Authorization},
    http::{Method, Request, Uri, StatusCode, self},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router, Server, TypedHeader,
};
use chrono::Utc;
use clap::Parser;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, Row, SqliteConnection};
use std::net::SocketAddr;
use std::{str::FromStr, sync::Arc};
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

mod error;

#[derive(Parser)]
struct CLI {
    #[clap(short, long, env = "LISTEN_ADDR", default_value = "0.0.0.0:8080")]
    listen_addr: String,

    /// Server authorization token
    #[clap(short, long, env = "ENDPOINT_TOKEN")]
    token: String,
}

// FIXME: full body accumulation
async fn ingestion(mut body: BodyStream) -> impl IntoResponse {
    let mut buf = vec![];
    while let Some(chunk) = body.next().await {
        buf.append(&mut chunk.unwrap().to_vec());
    }
    let reader = StreamReader::try_new(buf.as_slice(), None).unwrap();
    for record_batch in reader {
        if let Ok(record_batch) = record_batch {
            println!(
                "got new record batch:\n{}",
                pretty_format_batches(&[record_batch]).unwrap()
            );
        }
    }
    ""
}

#[derive(Serialize, Deserialize, Debug)]
struct UIConfig {
    ui_metadata: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Configs {
    configs: Vec<PipeConfig>,
    ui_metadata: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PipeConfig {
    id: u32,
    pipe: serde_json::Value,
}

async fn get_pipe_configs(State(app): State<Arc<App>>) -> Result<impl IntoResponse, error::Error> {
    Ok(app.get_configs().await.map(Json)?)
}

async fn post_pipe_config(
    State(app): State<Arc<App>>,
    Json(configs): Json<Configs>,
) -> Result<impl IntoResponse, error::Error> {
    app.set_configs(configs).await
}

async fn basic_auth<B>(
    State(app): State<Arc<App>>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let auth_header = req.headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    match auth_header {
        Some(auth_header)  => {
            match app.basic_auth(auth_header) {
                true => Ok(next.run(req).await),
                false => Err(StatusCode::UNAUTHORIZED),
            }
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
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

    let error: Option<&error::Error> = response.extensions().get();
    // FIXME: do not log token
    let log = json!({
        "token": token,
        "timestamp": timestamp,
        "request_time_ms": request_time_ms,
        "method": method.as_str(),
        "status_code": response.status().as_u16(),
        "path": uri.path(),
        "error": error.map(|e| format!("{:?}", e)),
    });
    println!("{}", log);
    response
}

#[derive(Deserialize, Serialize)]
struct ProvisionClientRequest {
    id: String,
}

async fn get_ui_metadata(State(app): State<Arc<App>>) -> Result<impl IntoResponse, error::Error> {
    Ok(app.get_ui_metadata().await.map(Json)?)
}

async fn provision_client(
    State(state): State<Arc<App>>,
    Json(payload): Json<ProvisionClientRequest>,
) -> Result<impl IntoResponse, error::Error> {
    let client_id = payload.id;

    state
        .database
        .insert_client(&client_id)
        .await
        .map(|_| Json(json!({ "id": client_id })))
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
) -> Result<impl IntoResponse, error::Error> {
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
            Ok(Json(response))
        }
        Err(e) => Err(e),
    }
}

#[derive(Debug)]
#[allow(unused)]
pub struct Database {
    connection: Arc<Mutex<SqliteConnection>>,
    database_path: String,
}

impl Database {
    async fn new(database_dir: &str) -> Result<Self, error::Error> {
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

    async fn insert_client(&self, client_id: &str) -> Result<(), error::Error> {
        let mut connection = self.connection.lock().await;
        let _ = sqlx::query("INSERT INTO clients (id) VALUES (?)")
            .bind(client_id)
            .execute(&mut *connection)
            .await?;
        Ok(())
    }

    async fn insert_token(&self, client_id: &str, token: &str) -> Result<(), error::Error> {
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

    async fn insert_config(
        &self,
        id: &u32,
        config: &serde_json::Value,
    ) -> Result<(), error::Error> {
        let mut connection = self.connection.lock().await;
        // FIXME: unwrap
        let config: String = serde_json::to_string(config)?;
        let _ = sqlx::query("INSERT INTO configs (id, raw_config) VALUES (?, ?)")
            .bind(id)
            .bind(config)
            .execute(&mut *connection)
            .await?;
        Ok(())
    }

    async fn insert_ui_metadata(
        &self,
        ui_metadata: &Option<serde_json::Value>,
    ) -> Result<(), error::Error> {
        let ui_metadata = match ui_metadata {
            Some(ui_metadata) => ui_metadata,
            None => return Ok(()),
        };
        let mut connection = self.connection.lock().await;

        // FIXME: unwrap
        let config: String = serde_json::to_string(ui_metadata).unwrap();
        let _ = sqlx::query("INSERT INTO ui_metadata (raw_config) VALUES (?)")
            .bind(config)
            .execute(&mut *connection)
            .await?;
        Ok(())
    }

    async fn get_ui_metadata(&self) -> Result<UIConfig, error::Error> {
        let mut connection = self.connection.lock().await;
        let row =
            sqlx::query("SELECT raw_config FROM ui_metadata ORDER BY created_at DESC LIMIT 1")
                .fetch_one(&mut *connection)
                .await?;
        // FIXME: get() will panic if column not presented
        let raw: String = row.get("raw_config");
        let ui_metadata: serde_json::Value = serde_json::from_str(raw.as_str()).unwrap();
        Ok(UIConfig {
            ui_metadata: Some(ui_metadata),
        })
    }

    async fn get_configs(&self) -> Result<Configs, error::Error> {
        let mut connection = self.connection.lock().await;
        // FIXME: sqlx allows query_as<Struct>
        let rows =
            sqlx::query("SELECT id, raw_config FROM configs GROUP BY id HAVING MAX(created_at)")
                .fetch_all(&mut *connection)
                .await?;

        let configs: Configs = Configs {
            configs: rows
                .into_iter()
                .map(|row| {
                    // FIXME: get() will panic if column not presented
                    let id: u32 = row.get("id");
                    let raw_config: String = row.get("raw_config");
                    let pipe: serde_json::Value =
                        serde_json::from_str(raw_config.as_str()).unwrap();
                    PipeConfig {
                        id,
                        pipe: serde_json::json!({ "section": pipe }),
                    }
                })
                .collect(),
            ui_metadata: None,
        };
        Ok(configs)
    }
}

#[derive(Debug)]
pub struct App {
    database: Database,
    token: String,
}

impl App {
    /// Set pipe configs
    async fn set_configs(&self, new_configs: Configs) -> Result<(), error::Error> {
        for config in new_configs.configs {
            self.database
                .insert_config(&config.id, &config.pipe)
                .await?
        }

        self.database
            .insert_ui_metadata(&new_configs.ui_metadata)
            .await?;
        Ok(())
    }

    /// Return pipe configs
    async fn get_configs(&self) -> Result<Configs, error::Error> {
        self.database.get_configs().await
    }

    async fn get_ui_metadata(&self) -> Result<UIConfig, error::Error> {
        self.database.get_ui_metadata().await
    }
}

impl App {
    pub async fn new(
        database_dir: impl AsRef<str>,
        token: impl Into<String>,
    ) -> anyhow::Result<Self> {
        tokio::fs::create_dir_all(database_dir.as_ref()).await?;
        let database = Database::new(database_dir.as_ref()).await?;
        Ok(Self {
            database,
            token: token.into(),
        })
    }

    fn basic_auth(&self, token: &str) -> bool {
        token == self.basic_auth_token()
    }

    fn basic_auth_token(&self) -> String {
        format!("Basic {}", BASE64.encode(format!("{}:", self.token)))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = CLI::try_parse()?;
    let app = App::new("", cli.token).await?;
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
        .route("/api/ui-metadata", get(get_ui_metadata))
        .layer(middleware::from_fn_with_state(state.clone(), basic_auth))
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
