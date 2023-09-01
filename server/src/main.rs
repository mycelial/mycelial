//! Mycelial server
use arrow::{
    ipc::{reader::StreamReader, writer::StreamWriter},
    record_batch::RecordBatch,
};
use axum::{
    extract::{BodyStream, State},
    headers::{authorization::Basic, Authorization},
    http::{self, header, Method, Request, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, get_service, post},
    Json, Router, Server, TypedHeader,
};
use base64::engine::{general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use clap::Parser;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, Row, SqliteConnection};
use std::{net::SocketAddr, path::Path};
use std::{str::FromStr, sync::Arc};
use tokio::sync::Mutex;
use tower_http::services::ServeDir;
use uuid::Uuid;
mod error;

#[derive(Parser)]
struct CLI {
    #[clap(short, long, env = "LISTEN_ADDR", default_value = "0.0.0.0:8080")]
    listen_addr: String,

    /// Server authorization token
    #[clap(short, long, env = "ENDPOINT_TOKEN")]
    token: String,

    /// Assets dir
    #[clap(short, long, env = "ASSETS_DIR")]
    assets_dir: String,

    /// Database path
    #[clap(short, long, env = "DATABASE_PATH", default_value = "mycelial.db")]
    database_path: String,
}

// FIXME: full body accumulation
async fn ingestion(
    State(app): State<Arc<App>>,
    axum::extract::Path(topic): axum::extract::Path<String>,
    headers: axum::http::header::HeaderMap,
    mut body: BodyStream,
) -> Result<impl IntoResponse, error::Error> {
    let origin = match headers.get("x-message-origin") {
        Some(origin) => origin
            .to_str()
            .map_err(|_| "bad x-message-origin header value")?,
        None => Err(StatusCode::BAD_REQUEST)?,
    };

    let mut buf = vec![];
    while let Some(chunk) = body.next().await {
        buf.append(&mut chunk.unwrap().to_vec());
    }
    let reader = StreamReader::try_new(buf.as_slice(), None).unwrap();
    for record_batch in reader {
        if let Ok(record_batch) = record_batch {
            app.database
                .store_record(&topic, origin, &record_batch)
                .await
                .unwrap()
        }
    }
    Ok(Json("ok"))
}

async fn get_record(
    State(app): State<Arc<App>>,
    axum::extract::Path((topic, offset)): axum::extract::Path<(String, i64)>,
) -> Result<impl IntoResponse, error::Error> {
    let response = match app.database.get_record(&topic, offset).await? {
        Some((id, origin, data)) => (
            [
                ("x-message-id", id.to_string()),
                ("x-message-origin", origin),
            ],
            data,
        ),
        None => (
            [
                ("x-message-id", offset.to_string()),
                ("x-message-origin", String::new()),
            ],
            vec![],
        ),
    };
    Ok(response)
}

#[derive(Serialize, Deserialize, Debug)]
struct Configs {
    configs: Vec<PipeConfig>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Clients {
    clients: Vec<Client>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Client {
    id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct PipeConfig {
    id: u32,
    pipe: serde_json::Value,
}

async fn get_pipe_configs(State(app): State<Arc<App>>) -> Result<impl IntoResponse, error::Error> {
    Ok(app.get_configs().await.map(Json)?)
}

async fn get_pipe_config(
    State(app): State<Arc<App>>,
    axum::extract::Path(id): axum::extract::Path<u32>,
) -> Result<impl IntoResponse, error::Error> {
    Ok(app.get_config(&id).await.map(Json)?)
}

async fn post_pipe_config(
    State(app): State<Arc<App>>,
    Json(configs): Json<Configs>,
) -> Result<impl IntoResponse, error::Error> {
    log::trace!("Configs in: {:?}", &configs);
    let ids = app.set_configs(&configs).await?;
    Ok(Json(
        ids.iter()
            .zip(configs.configs)
            .map(|(id, conf)| PipeConfig {
                id: *id,
                pipe: conf.pipe,
            })
            .collect::<Vec<PipeConfig>>(),
    )
    .into_response())
}

async fn put_pipe_configs(
    State(app): State<Arc<App>>,
    Json(configs): Json<Configs>,
) -> Result<impl IntoResponse, error::Error> {
    Ok(app.update_configs(configs).await.map(Json)?)
}

async fn put_pipe_config(
    State(app): State<Arc<App>>,
    axum::extract::Path(id): axum::extract::Path<u32>,
    Json(mut config): Json<PipeConfig>,
) -> Result<impl IntoResponse, error::Error> {
    config.id = id;
    Ok(app.update_config(config).await.map(Json)?)
}

async fn delete_pipe_config(
    State(app): State<Arc<App>>,
    axum::extract::Path(id): axum::extract::Path<u32>,
) -> Result<impl IntoResponse, error::Error> {
    app.delete_config(&id).await
}

async fn get_clients(State(app): State<Arc<App>>) -> Result<impl IntoResponse, error::Error> {
    Ok(app.database.get_clients().await.map(Json)?)
}

async fn basic_auth<B>(
    State(app): State<Arc<App>>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, impl IntoResponse> {
    let auth_header = req
        .headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    match auth_header {
        Some(auth_header) if app.basic_auth(auth_header) => return Ok(next.run(req).await),
        _ => (),
    };
    let response = (
        [(header::WWW_AUTHENTICATE, "Basic")],
        StatusCode::UNAUTHORIZED,
    );
    Err(response)
}

async fn client_auth<B>(
    State(_app): State<Arc<App>>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get("X-Authorization")
        .and_then(|header| header.to_str().ok());

    match auth_header {
        Some(_auth_header) => Ok(next.run(req).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

// log response middleware
async fn log_middleware<B>(
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
    log::info!("{}", log);
    response
}

#[derive(Deserialize, Serialize)]
struct ProvisionClientRequest {
    id: String,
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
    async fn new(database_path: &str) -> Result<Self, error::Error> {
        let database_url = format!("sqlite://{database_path}");
        let mut connection = SqliteConnectOptions::from_str(database_url.as_str())?
            .create_if_missing(true)
            .connect()
            .await?;
        sqlx::migrate!().run(&mut connection).await?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            database_path: database_path.into(),
        })
    }

    async fn insert_client(&self, client_id: &str) -> Result<(), error::Error> {
        let mut connection = self.connection.lock().await;
        let _ = sqlx::query("INSERT INTO clients (id) VALUES (?) ON CONFLICT (id) DO NOTHING")
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

    async fn insert_config(&self, config: &serde_json::Value) -> Result<i64, error::Error> {
        let mut connection = self.connection.lock().await;
        // FIXME: unwrap
        let config: String = serde_json::to_string(config)?;
        let id = sqlx::query("INSERT INTO pipes (raw_config) VALUES (?)")
            .bind(config)
            .execute(&mut *connection)
            .await?
            .last_insert_rowid();
        Ok(id)
    }

    async fn update_config(
        &self,
        id: &u32,
        config: &serde_json::Value,
    ) -> Result<(), error::Error> {
        let mut connection = self.connection.lock().await;
        let config: String = serde_json::to_string(config)?;
        let _ = sqlx::query("update pipes set raw_config = ? WHERE id = ?")
            .bind(config)
            .bind(id)
            .execute(&mut *connection)
            .await?;
        Ok(())
    }

    async fn delete_config(&self, id: &u32) -> Result<(), error::Error> {
        let mut connection = self.connection.lock().await;
        let _ = sqlx::query("DELETE FROM pipes WHERE id = ?")
            .bind(id)
            .execute(&mut *connection)
            .await?;
        Ok(())
    }

    async fn store_record(
        &self,
        topic: &str,
        origin: &str,
        record_batch: &RecordBatch,
    ) -> Result<(), error::Error> {
        let mut stream_writer: StreamWriter<_> =
            StreamWriter::try_new(vec![], record_batch.schema().as_ref()).unwrap();
        // FIXME: unwrap
        stream_writer.write(record_batch).unwrap();
        stream_writer.finish().unwrap();

        let bytes: Vec<u8> = stream_writer.into_inner().unwrap().into();

        let mut connection = self.connection.lock().await;
        sqlx::query("INSERT INTO records (topic, origin, data) VALUES (?, ?, ?)")
            .bind(topic)
            .bind(origin)
            .bind(bytes)
            .execute(&mut *connection)
            .await?;
        Ok(())
    }

    async fn get_record(
        &self,
        topic: &str,
        offset: i64,
    ) -> Result<Option<(i64, String, Vec<u8>)>, error::Error> {
        let mut connection = self.connection.lock().await;
        let row = sqlx::query(
            "SELECT id, origin, data FROM records WHERE topic = ? AND id > ? ORDER BY id ASC LIMIT 1",
        )
            .bind(topic)
            .bind(offset)
            .fetch_optional(&mut *connection)
            .await?;
        Ok(row.map(|row| (row.get("id"), row.get("origin"), row.get("data"))))
    }

    async fn get_clients(&self) -> Result<Clients, error::Error> {
        let mut connection = self.connection.lock().await;
        let rows = sqlx::query("SELECT id FROM clients")
            .fetch_all(&mut *connection)
            .await?;

        let clients = Clients {
            clients: rows
                .into_iter()
                .map(|row| {
                    let id = row.get("id");
                    Client { id }
                })
                .collect(),
        };
        Ok(clients)
    }

    async fn get_config(&self, id: &u32) -> Result<PipeConfig, error::Error> {
        let mut connection = self.connection.lock().await;
        let row = sqlx::query("SELECT id, raw_config from pipes WHERE id = ?")
            .bind(id)
            .fetch_one(&mut *connection)
            .await?;
        let id: u32 = row.get("id");
        let raw_config: String = row.get("raw_config");
        Ok(PipeConfig {
            id,
            pipe: serde_json::json!({ "section": raw_config }),
        })
    }

    async fn get_configs(&self) -> Result<Configs, error::Error> {
        let mut connection = self.connection.lock().await;
        // FIXME: sqlx allows query_as<Struct>
        let rows = sqlx::query("SELECT id, raw_config from pipes")
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
    async fn delete_config(&self, id: &u32) -> Result<(), error::Error> {
        self.database.delete_config(id).await?;
        Ok(())
    }

    /// Set pipe configs
    async fn set_configs(&self, new_configs: &Configs) -> Result<Vec<u32>, error::Error> {
        let mut inserted_ids = Vec::new();

        for config in new_configs.configs.iter() {
            let id = self.database.insert_config(&config.pipe).await?;
            inserted_ids.push(id as u32);
        }

        Ok(inserted_ids)
    }

    async fn update_configs(&self, configs: Configs) -> Result<(), error::Error> {
        for config in configs.configs {
            self.database
                .update_config(&config.id, &config.pipe)
                .await?
        }
        Ok(())
    }

    async fn update_config(&self, config: PipeConfig) -> Result<PipeConfig, error::Error> {
        self.database
            .update_config(&config.id, &config.pipe)
            .await?;
        Ok(config)
    }

    async fn get_config(&self, id: &u32) -> Result<PipeConfig, error::Error> {
        self.database.get_config(id).await
    }

    /// Return pipe configs
    async fn get_configs(&self) -> Result<Configs, error::Error> {
        self.database.get_configs().await
    }
}

impl App {
    pub async fn new(db_path: impl AsRef<str>, token: impl Into<String>) -> anyhow::Result<Self> {
        let db_path: &str = db_path.as_ref();
        if let Some(parent) = Path::new(db_path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        };
        let database = Database::new(db_path).await?;
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
    pretty_env_logger::init();

    let cli = CLI::try_parse()?;
    let app = App::new(cli.database_path, cli.token).await?;
    let state = Arc::new(app);

    // FIXME: consistent endpoint namings
    let api = Router::new()
        .route("/api/client", post(provision_client)) // no client auth needed
        .route("/api/tokens", post(issue_token)) // no client auth needed
        .route("/ingestion/:topic", post(ingestion))
        .route("/ingestion/:topic/:offset", get(get_record))
        .merge(
            Router::new()
                .route(
                    "/api/pipe/configs",
                    get(get_pipe_configs)
                        .post(post_pipe_config)
                        .put(put_pipe_configs),
                )
                .route(
                    "/api/pipe/configs/:id",
                    get(get_pipe_config)
                        .delete(delete_pipe_config)
                        .put(put_pipe_config),
                )
                .route("/api/clients", get(get_clients))
                .layer(middleware::from_fn_with_state(state.clone(), client_auth)),
        )
        .with_state(state.clone());

    let assets = Router::new().nest_service("/", get_service(ServeDir::new(cli.assets_dir)));

    let router = Router::new()
        .merge(api)
        .merge(assets)
        .layer(middleware::from_fn_with_state(state.clone(), basic_auth))
        .layer(middleware::from_fn(log_middleware));

    let addr: SocketAddr = cli.listen_addr.as_str().parse()?;
    Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;
    Ok(())
}
