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
    routing::{get, post},
    Json, Router, Server, TypedHeader,
};
use base64::engine::{general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Utc};
use clap::Parser;
use common::{
    Destination, IssueTokenRequest, IssueTokenResponse, PipeConfig, PipeConfigs,
    ProvisionClientRequest, ProvisionClientResponse, Source,
};
use futures::StreamExt;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{
    sqlite::SqliteConnectOptions, sqlite::SqliteRow, ConnectOptions, FromRow, Row, SqliteConnection,
};
use std::{net::SocketAddr, path::Path};
use std::{str::FromStr, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

mod error;

#[derive(Serialize, Deserialize, Debug)]
pub struct Workspace {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub pipe_configs: Vec<PipeConfig>,
    pub name: String,
}

impl FromRow<'_, SqliteRow> for Workspace {
    fn from_row(row: &SqliteRow) -> sqlx::Result<Self> {
        Ok(Self {
            id: row.get("id"),
            name: row.get("name"),
            created_at: Utc::now(),
            // created_at: row.get("created_at"),
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
    for record_batch in reader.flatten() {
        app.database
            .store_record(&topic, origin, &record_batch)
            .await
            .unwrap()
    }
    Ok(Json("ok"))
}

async fn get_record(
    State(app): State<Arc<App>>,
    axum::extract::Path((topic, offset)): axum::extract::Path<(String, u64)>,
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

async fn get_pipe_configs(State(app): State<Arc<App>>) -> Result<impl IntoResponse, error::Error> {
    app.get_configs().await.map(Json)
}

async fn get_pipe_config(
    State(app): State<Arc<App>>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<impl IntoResponse, error::Error> {
    app.get_config(id).await.map(Json)
}

// save a name and get an id assigned. it's a place to create pipes in
async fn create_workspace(
    State(app): State<Arc<App>>,
    Json(workspace): Json<Workspace>,
) -> Result<impl IntoResponse, error::Error> {
    app.create_workspace(workspace).await.map(Json)
}

// gets a list of all the workspaces, ids, names, etc. not hydrated with pipe configs
async fn get_workspaces(State(app): State<Arc<App>>) -> Result<impl IntoResponse, error::Error> {
    app.get_workspaces().await.map(Json)
}

// by id, fetches a workspaces, hydrated with the pipe configs
async fn get_workspace(
    State(app): State<Arc<App>>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<impl IntoResponse, error::Error> {
    app.get_workspace(id).await.map(Json)
}

// updates a workspace. updating what workspace a pipe is part of is done by updating the pipe config
async fn update_workspace(
    State(app): State<Arc<App>>,
    axum::extract::Path(id): axum::extract::Path<u64>,
    Json(mut workspace): Json<Workspace>,
) -> Result<impl IntoResponse, error::Error> {
    let id: i64 = id.try_into().unwrap();
    workspace.id = id;
    app.update_workspace(workspace).await.map(Json)
}

// deletes a workspace by id
async fn delete_workspace(
    State(app): State<Arc<App>>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<impl IntoResponse, error::Error> {
    app.delete_workspace(id).await
}

async fn post_pipe_config(
    State(app): State<Arc<App>>,
    Json(configs): Json<PipeConfigs>,
) -> Result<impl IntoResponse, error::Error> {
    log::trace!("Configs in: {:?}", &configs);
    let ids = app.set_configs(&configs).await?;
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

async fn put_pipe_configs(
    State(app): State<Arc<App>>,
    Json(configs): Json<PipeConfigs>,
) -> Result<impl IntoResponse, error::Error> {
    app.update_configs(configs).await.map(Json)
}

async fn put_pipe_config(
    State(app): State<Arc<App>>,
    axum::extract::Path(id): axum::extract::Path<u64>,
    Json(mut config): Json<PipeConfig>,
) -> Result<impl IntoResponse, error::Error> {
    config.id = id;
    app.update_config(config).await.map(Json)
}

async fn delete_pipe_config(
    State(app): State<Arc<App>>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<impl IntoResponse, error::Error> {
    app.delete_config(id).await
}

async fn get_clients(State(app): State<Arc<App>>) -> Result<impl IntoResponse, error::Error> {
    app.database.get_clients().await.map(Json)
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

// async fn client_auth<B>(
//     State(_app): State<Arc<App>>,
//     req: Request<B>,
//     next: Next<B>,
// ) -> Result<Response, StatusCode> {
//     let auth_header = req
//         .headers()
//         .get("X-Authorization")
//         .and_then(|header| header.to_str().ok());

//     match auth_header {
//         Some(_auth_header) => Ok(next.run(req).await),
//         _ => Err(StatusCode::UNAUTHORIZED),
//     }
// }

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

async fn provision_client(
    State(state): State<Arc<App>>,
    Json(payload): Json<ProvisionClientRequest>,
) -> Result<impl IntoResponse, error::Error> {
    let client_id = payload.client_config.node.unique_id;

    state
        .database
        .insert_client(
            &client_id,
            &payload.client_config.node.display_name,
            &payload.client_config.sources,
            &payload.client_config.destinations,
        )
        .await
        .map(|_| {
            Json(ProvisionClientResponse {
                id: client_id.clone(),
            })
        })
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

    async fn insert_client(
        &self,
        client_id: &str,
        display_name: &str,
        sources: &[Source],
        destinations: &[Destination],
    ) -> Result<(), error::Error> {
        let sources = serde_json::to_string(sources)?;
        let destinations = serde_json::to_string(destinations)?;

        let mut connection = self.connection.lock().await;
        let _ = sqlx::query("INSERT OR REPLACE INTO clients (id, display_name, sources, destinations) VALUES (?, ?, ?, ?)")
            .bind(client_id)
            .bind(display_name)
            .bind(sources)
            .bind(destinations)
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
        config: &serde_json::Value,
        workspace_id: i64,
    ) -> Result<u64, error::Error> {
        let mut connection = self.connection.lock().await;
        // FIXME: unwrap
        let config: String = serde_json::to_string(config)?;
        let id = sqlx::query("INSERT INTO pipes (raw_config, workspace_id) VALUES (?, ?)")
            .bind(config)
            .bind(workspace_id)
            .execute(&mut *connection)
            .await?
            .last_insert_rowid();
        Ok(id.try_into().unwrap())
    }

    async fn update_config(&self, id: u64, config: &serde_json::Value) -> Result<(), error::Error> {
        let mut connection = self.connection.lock().await;
        let config: String = serde_json::to_string(config)?;
        let id: i64 = id.try_into().unwrap();
        let _ = sqlx::query("update pipes set raw_config = ? WHERE id = ?")
            .bind(config)
            .bind(id)
            .execute(&mut *connection)
            .await?;
        Ok(())
    }

    async fn delete_config(&self, id: u64) -> Result<(), error::Error> {
        let mut connection = self.connection.lock().await;
        let id: i64 = id.try_into().unwrap();
        let _ = sqlx::query("DELETE FROM pipes WHERE id = ?")
            .bind(id) // fixme
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

        let bytes: Vec<u8> = stream_writer.into_inner().unwrap();

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
        offset: u64,
    ) -> Result<Option<(u64, String, Vec<u8>)>, error::Error> {
        let mut connection = self.connection.lock().await;
        let offset: i64 = offset.try_into().unwrap();
        let row = sqlx::query(
            "SELECT id, origin, data FROM records WHERE topic = ? AND id > ? ORDER BY id ASC LIMIT 1",
        )
            .bind(topic)
            .bind(offset)
            .fetch_optional(&mut *connection)
            .await?;
        Ok(row.map(|row| {
            (
                row.get::<i64, &str>("id").try_into().unwrap(),
                row.get("origin"),
                row.get("data"),
            )
        }))
    }

    async fn get_clients(&self) -> Result<Clients, error::Error> {
        let mut connection = self.connection.lock().await;
        // todo: should we return ui as client?
        let rows = sqlx::query("SELECT id, display_name, sources, destinations FROM clients")
            .fetch_all(&mut *connection)
            .await?;

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

    async fn get_workspaces(&self) -> Result<Vec<Workspace>, error::Error> {
        let mut connection = self.connection.lock().await;
        let records: Vec<Workspace> =
            sqlx::query_as(r"SELECT id, name, created_at FROM workspaces")
                .fetch_all(&mut *connection)
                .await
                .unwrap();

        Ok(records)
    }

    async fn create_workspace(&self, mut workspace: Workspace) -> Result<Workspace, error::Error> {
        let mut connection = self.connection.lock().await;
        workspace.id = sqlx::query("INSERT INTO workspaces (name) VALUES (?)")
            .bind(workspace.name.clone())
            .execute(&mut *connection)
            .await?
            .last_insert_rowid();
        Ok(workspace)
    }

    async fn get_workspace(&self, id: u64) -> Result<Workspace, error::Error> {
        let mut connection = self.connection.lock().await;
        let id: i64 = id.try_into().unwrap();
        let mut record: Workspace =
            sqlx::query_as(r"SELECT id, name, created_at FROM workspaces WHERE id = ?")
                .bind(id)
                .fetch_one(&mut *connection)
                .await
                .unwrap();

        let pipes: Vec<PipeConfig> =
            sqlx::query_as("SELECT id, raw_config, workspace_id from pipes where workspace_id = ?")
                .bind(id)
                .fetch_all(&mut *connection)
                .await?;

        record.pipe_configs = pipes;

        Ok(record)
    }

    async fn update_workspace(&self, workspace: Workspace) -> Result<Workspace, error::Error> {
        let mut connection = self.connection.lock().await;
        let _ = sqlx::query("UPDATE workspaces SET name = ? where id = ?")
            .bind(workspace.name.clone())
            .bind(workspace.id)
            .execute(&mut *connection)
            .await
            .unwrap();
        Ok(workspace)
    }

    async fn delete_workspace(&self, id: u64) -> Result<(), error::Error> {
        let mut connection = self.connection.lock().await;
        let id: i64 = id.try_into().unwrap();
        let _ = sqlx::query("DELETE FROM pipes WHERE id = ?")
            .bind(id) // fixme
            .execute(&mut *connection)
            .await?;
        Ok(())
    }

    async fn get_config(&self, id: u64) -> Result<PipeConfig, error::Error> {
        let mut connection = self.connection.lock().await;
        let id: i64 = id.try_into().unwrap();
        let pipe: PipeConfig =
            sqlx::query_as("SELECT id, workspace_id, raw_config from pipes WHERE id = ?")
                .bind(id)
                .fetch_one(&mut *connection)
                .await?;
        Ok(pipe)
    }

    async fn get_configs(&self) -> Result<PipeConfigs, error::Error> {
        let mut connection = self.connection.lock().await;
        let rows: Vec<PipeConfig> =
            sqlx::query_as("SELECT id, raw_config, workspace_id from pipes")
                .fetch_all(&mut *connection)
                .await?;

        let configs: PipeConfigs = PipeConfigs { configs: rows };
        Ok(configs)
    }
}

#[derive(Debug)]
pub struct App {
    database: Database,
    token: String,
}

#[derive(RustEmbed)]
#[folder = "../console/out/"]
pub struct Assets;

impl App {
    async fn delete_config(&self, id: u64) -> Result<(), error::Error> {
        self.database.delete_config(id).await?;
        Ok(())
    }

    /// Set pipe configs
    async fn set_configs(&self, new_configs: &PipeConfigs) -> Result<Vec<u64>, error::Error> {
        let mut inserted_ids = Vec::new();
        for config in new_configs.configs.iter() {
            let id = self
                .database
                .insert_config(&config.pipe, config.workspace_id.try_into().unwrap())
                .await?;
            inserted_ids.push(id);
        }
        Ok(inserted_ids)
    }

    async fn update_configs(&self, configs: PipeConfigs) -> Result<(), error::Error> {
        for config in configs.configs {
            self.database.update_config(config.id, &config.pipe).await?
        }
        Ok(())
    }

    async fn update_config(&self, config: PipeConfig) -> Result<PipeConfig, error::Error> {
        self.database.update_config(config.id, &config.pipe).await?;
        Ok(config)
    }

    async fn get_config(&self, id: u64) -> Result<PipeConfig, error::Error> {
        self.database.get_config(id).await
    }

    async fn get_workspaces(&self) -> Result<Vec<Workspace>, error::Error> {
        self.database.get_workspaces().await
    }

    async fn create_workspace(&self, workspace: Workspace) -> Result<Workspace, error::Error> {
        self.database.create_workspace(workspace).await
    }

    async fn get_workspace(&self, id: u64) -> Result<Workspace, error::Error> {
        self.database.get_workspace(id).await
    }

    async fn update_workspace(&self, workspace: Workspace) -> Result<Workspace, error::Error> {
        self.database.update_workspace(workspace).await
    }

    async fn delete_workspace(&self, id: u64) -> Result<(), error::Error> {
        self.database.delete_workspace(id).await
    }

    async fn get_configs(&self) -> Result<PipeConfigs, error::Error> {
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
        None => Err(StatusCode::NOT_FOUND),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();

    let cli = Cli::try_parse()?;
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
                    "/api/pipe",
                    get(get_pipe_configs)
                        .post(post_pipe_config)
                        .put(put_pipe_configs),
                )
                .route(
                    "/api/pipe/:id",
                    get(get_pipe_config)
                        .delete(delete_pipe_config)
                        .put(put_pipe_config),
                )
                .route(
                    "/api/workspaces/:id",
                    get(get_workspace)
                        .put(update_workspace)
                        .delete(delete_workspace),
                )
                .route(
                    "/api/workspaces",
                    get(get_workspaces).post(create_workspace),
                )
                .route("/api/clients", get(get_clients)),
        )
        .with_state(state.clone());

    let assets = Router::new().fallback(assets);

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
