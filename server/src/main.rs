use axum::{
    body::StreamBody,
    extract::{BodyStream, State},
    headers::{authorization::Basic, Authorization},
    http::{self, header, Method, Request, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router, Server, TypedHeader,
};
use base64::engine::{general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Utc};
use clap::Parser;
use common::{
    Destination, PipeConfig, PipeConfigs, ProvisionClientRequest, ProvisionClientResponse, Source,
};
use futures::{Stream, StreamExt};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{
    sqlite::SqliteConnectOptions, sqlite::SqliteRow, ConnectOptions, Connection, FromRow, Row,
    Sqlite, SqliteConnection, Transaction,
};
use std::pin::Pin;
use std::{net::SocketAddr, path::Path};
use std::{str::FromStr, sync::Arc};
use tokio::sync::{Mutex, MutexGuard};
use uuid::Uuid;

mod error;

use jsonwebtoken::{decode, jwk, DecodingKey, Validation};

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

// middleware that checks for the token in the request and associates it with a client/daemon
async fn daemon_auth<B>(
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
                [client_id, client_secret] => {
                    Some((client_id.to_string(), client_secret.to_string()))
                }
                _ => None,
            }
        });

    if let Some((client_id, client_secret)) = decoded {
        let user_id = app
            .validate_client_id_and_secret(client_id.as_str(), client_secret.as_str())
            .await;
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
    let response = (StatusCode::UNAUTHORIZED, "invalid token");
    Err(response)
}

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
            created_at: Utc::now(), // TODO: get from db
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

    #[clap(short, long, env = "AUTH0_AUTHORITY")]
    auth0_authority: String,

    #[clap(short, long, env = "AUTH0_AUDIENCE")]
    auth0_audience: String,
}

struct MessageStream {
    id: u64,
    origin: String,
    stream_type: String,
    stream: Pin<Box<dyn Stream<Item = Result<Vec<u8>, error::Error>> + Send>>,
}

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

    let stream_type = match headers.get("x-stream-type") {
        Some(origin) => origin
            .to_str()
            .map_err(|_| "bad x-message-origin header value")?,
        None => "dataframe", // by default
    };

    let mut connection = app.database.get_connection().await;
    let mut transaction = connection.begin().await?;

    let message_id = app
        .database
        .new_message(&mut transaction, topic.as_str(), origin, stream_type)
        .await?;

    let mut stored = 0;
    while let Some(chunk) = body.next().await {
        // FIXME: accumulate into buffer
        let chunk = chunk?;
        app.database
            .store_chunk(&mut transaction, message_id, chunk.as_ref())
            .await?;
        stored += 1;
    }
    // don't store empty messages
    match stored {
        0 => transaction.rollback().await?,
        _ => transaction.commit().await?,
    };
    Ok(Json("ok"))
}

async fn get_message(
    State(app): State<Arc<App>>,
    axum::extract::Path((topic, offset)): axum::extract::Path<(String, u64)>,
) -> Result<impl IntoResponse, error::Error> {
    let response = match app.database.get_message(&topic, offset).await? {
        None => {
            let stream: Pin<Box<dyn Stream<Item = _> + Send>> =
                Box::pin(futures::stream::empty::<Result<Vec<u8>, error::Error>>());
            (
                [
                    ("x-message-id", offset.to_string()),
                    ("x-message-origin", "".into()),
                    ("x-stream-type", "".into()),
                ],
                StreamBody::new(stream),
            )
        }
        Some(MessageStream {
            id,
            origin,
            stream_type,
            stream,
        }) => (
            [
                ("x-message-id", id.to_string()),
                ("x-message-origin", origin),
                ("x-stream-type", stream_type),
            ],
            StreamBody::new(stream),
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

async fn get_pipe_configs(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
) -> Result<impl IntoResponse, error::Error> {
    app.get_configs(user_id.0.as_str()).await.map(Json)
}

async fn get_pipe_config(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<impl IntoResponse, error::Error> {
    app.get_config(id, user_id.0.as_str()).await.map(Json)
}

// save a name and get an id assigned. it's a place to create pipes in
async fn create_workspace(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    Json(workspace): Json<Workspace>,
) -> Result<impl IntoResponse, error::Error> {
    app.create_workspace(workspace, user_id.0.as_str())
        .await
        .map(Json)
}

// gets a list of all the workspaces, ids, names, etc. not hydrated with pipe configs
async fn get_workspaces(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
) -> Result<impl IntoResponse, error::Error> {
    app.get_workspaces(user_id.0.as_str()).await.map(Json)
}

// by id, fetches a workspaces, hydrated with the pipe configs
async fn get_workspace(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<impl IntoResponse, error::Error> {
    app.get_workspace(id, user_id.0.as_str()).await.map(Json)
}

// updates a workspace. updating what workspace a pipe is part of is done by updating the pipe config
async fn update_workspace(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    axum::extract::Path(id): axum::extract::Path<u64>,
    Json(mut workspace): Json<Workspace>,
) -> Result<impl IntoResponse, error::Error> {
    let id: i64 = id.try_into().unwrap();
    workspace.id = id;
    app.update_workspace(workspace, user_id.0.as_str())
        .await
        .map(Json)
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

// deletes a workspace by id
async fn delete_workspace(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<impl IntoResponse, error::Error> {
    app.delete_workspace(id, user_id.0.as_str()).await
}

async fn post_pipe_config(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    Json(configs): Json<PipeConfigs>,
) -> Result<impl IntoResponse, error::Error> {
    log::trace!("Configs in: {:?}", &configs);
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

async fn put_pipe_configs(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    Json(configs): Json<PipeConfigs>,
) -> Result<impl IntoResponse, error::Error> {
    app.validate_configs(&configs)?;
    app.update_configs(configs, user_id.0.as_str())
        .await
        .map(Json)
}

async fn put_pipe_config(
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

async fn delete_pipe_config(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<impl IntoResponse, error::Error> {
    app.delete_config(id, user_id.0.as_str()).await
}

async fn get_clients(
    State(app): State<Arc<App>>,
    Extension(user_id): Extension<UserID>,
) -> Result<impl IntoResponse, error::Error> {
    app.database.get_clients(user_id.0.as_str()).await.map(Json)
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

async fn provision_client(
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

    #[allow(clippy::too_many_arguments)]
    async fn insert_client(
        &self,
        client_id: &str,
        user_id: &str,
        display_name: &str,
        sources: &[Source],
        destinations: &[Destination],
        unique_client_id: &str,
        client_secret_hash: &str,
    ) -> Result<(), error::Error> {
        let sources = serde_json::to_string(sources)?;
        let destinations = serde_json::to_string(destinations)?;

        let mut connection = self.connection.lock().await;
        let _ = sqlx::query("INSERT OR REPLACE INTO clients (id, user_id, display_name, sources, destinations, unique_client_id, client_secret_hash) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(client_id)
            .bind(user_id)
            .bind(display_name)
            .bind(sources)
            .bind(destinations)
            .bind(unique_client_id)
            .bind(client_secret_hash)
            .execute(&mut *connection)
            .await?;
        Ok(())
    }

    async fn insert_config(
        &self,
        config: &serde_json::Value,
        workspace_id: i64,
        user_id: &str,
    ) -> Result<u64, error::Error> {
        let mut connection = self.connection.lock().await;
        let config: String = serde_json::to_string(config)?;
        let id =
            sqlx::query("INSERT INTO pipes (raw_config, workspace_id, user_id) VALUES (?, ?, ?)")
                .bind(config)
                .bind(workspace_id)
                .bind(user_id)
                .execute(&mut *connection)
                .await?
                .last_insert_rowid();
        // FIXME: unwrap
        Ok(id.try_into().unwrap())
    }

    async fn update_config(
        &self,
        id: u64,
        config: &serde_json::Value,
        user_id: &str,
    ) -> Result<(), error::Error> {
        let mut connection = self.connection.lock().await;
        let config: String = serde_json::to_string(config)?;
        // FIXME: unwrap
        let id: i64 = id.try_into().unwrap();
        let _ = sqlx::query("update pipes set raw_config = ? WHERE id = ? and user_id = ?")
            .bind(config)
            .bind(id)
            .bind(user_id)
            .execute(&mut *connection)
            .await?;
        Ok(())
    }

    async fn delete_config(&self, id: u64, user_id: &str) -> Result<(), error::Error> {
        let mut connection = self.connection.lock().await;
        // FIXME: unwrap
        let id: i64 = id.try_into().unwrap();
        let _ = sqlx::query("DELETE FROM pipes WHERE id = ? and user_id = ?")
            .bind(id) // fixme
            .bind(user_id)
            .execute(&mut *connection)
            .await?;
        Ok(())
    }

    async fn get_connection(&self) -> MutexGuard<'_, SqliteConnection> {
        self.connection.lock().await
    }

    async fn new_message(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        topic: &str,
        origin: &str,
        stream_type: &str,
    ) -> Result<i64, error::Error> {
        let id = sqlx::query(
            "INSERT INTO messages(topic, origin, stream_type) VALUES(?, ?, ?) RETURNING ID",
        )
        .bind(topic)
        .bind(origin)
        .bind(stream_type)
        .fetch_one(&mut **transaction)
        .await
        .map(|row| row.get::<i64, _>(0))?;
        Ok(id)
    }

    async fn store_chunk(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
        message_id: i64,
        bytes: &[u8],
    ) -> Result<(), error::Error> {
        sqlx::query("INSERT INTO records (message_id, data) VALUES (?, ?)")
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
        let mut connection = Arc::clone(&self.connection).lock_owned().await;
        // FIXME: unwrap
        let offset: i64 = offset.try_into().unwrap();
        let message_info = sqlx::query(
            "SELECT id, origin, stream_type FROM messages WHERE id > ? and topic = ? LIMIT 1",
        )
        .bind(offset)
        .bind(topic)
        .fetch_optional(&mut *connection)
        .await?
        .map(|row| {
            (
                row.get::<i64, _>(0) as u64,
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
            let mut stream = sqlx::query("SELECT data FROM records r WHERE r.message_id = ?")
                .bind(id as i64)
                .fetch(&mut *connection)
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
        let mut connection = self.connection.lock().await;
        // todo: should we return ui as client?
        let mut query = sqlx::query("SELECT id, display_name, sources, destinations FROM clients");
        if cfg!(feature = "require_auth") {
            query = sqlx::query(
                "SELECT id, display_name, sources, destinations FROM clients where user_id = ?",
            )
            .bind(user_id);
        }
        let rows = query.fetch_all(&mut *connection).await?;

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
        let mut connection = self.connection.lock().await;
        let records: Vec<Workspace> =
            sqlx::query_as(r"SELECT id, name, created_at FROM workspaces where user_id = ?")
                .bind(user_id)
                .fetch_all(&mut *connection)
                .await?;

        Ok(records)
    }

    async fn create_workspace(
        &self,
        mut workspace: Workspace,
        user_id: &str,
    ) -> Result<Workspace, error::Error> {
        let mut connection = self.connection.lock().await;
        workspace.id = sqlx::query("INSERT INTO workspaces (name, user_id) VALUES (?, ?)")
            .bind(workspace.name.clone())
            .bind(user_id)
            .execute(&mut *connection)
            .await?
            .last_insert_rowid();
        Ok(workspace)
    }

    async fn get_workspace(&self, id: u64, user_id: &str) -> Result<Workspace, error::Error> {
        let mut connection = self.connection.lock().await;
        let id: i64 = id.try_into().unwrap();
        let mut record: Workspace = sqlx::query_as(
            r"SELECT id, name, created_at FROM workspaces WHERE id = ? and user_id = ?",
        )
        .bind(id)
        .bind(user_id)
        .fetch_one(&mut *connection)
        .await?;

        let pipes: Vec<PipeConfig> = sqlx::query_as(
            "SELECT id, raw_config, workspace_id from pipes where workspace_id = ? and user_id = ?",
        )
        .bind(id)
        .bind(user_id)
        .fetch_all(&mut *connection)
        .await?;

        record.pipe_configs = pipes;

        Ok(record)
    }

    async fn update_workspace(
        &self,
        workspace: Workspace,
        user_id: &str,
    ) -> Result<Workspace, error::Error> {
        let mut connection = self.connection.lock().await;
        let _ = sqlx::query("UPDATE workspaces SET name = ? where id = ? and user_id = ?")
            .bind(workspace.name.clone())
            .bind(workspace.id)
            .bind(user_id)
            .execute(&mut *connection)
            .await?;
        Ok(workspace)
    }

    async fn delete_workspace(&self, id: u64, user_id: &str) -> Result<(), error::Error> {
        let mut connection = self.connection.lock().await;
        let id: i64 = id.try_into().unwrap();
        let _ = sqlx::query("DELETE FROM workspaces WHERE id = ? and user_id = ?")
            .bind(id) // fixme
            .bind(user_id)
            .execute(&mut *connection)
            .await?;
        Ok(())
    }

    async fn get_config(&self, id: u64, user_id: &str) -> Result<PipeConfig, error::Error> {
        let mut connection = self.connection.lock().await;
        let id: i64 = id.try_into().unwrap();
        let pipe: PipeConfig = sqlx::query_as(
            "SELECT id, workspace_id, raw_config from pipes WHERE id = ? and user_id = ?",
        )
        .bind(id)
        .bind(user_id)
        .fetch_one(&mut *connection)
        .await?;
        Ok(pipe)
    }

    async fn get_configs(&self, user_id: &str) -> Result<PipeConfigs, error::Error> {
        let mut connection = self.connection.lock().await;
        let rows: Vec<PipeConfig> =
            sqlx::query_as("SELECT id, raw_config, workspace_id from pipes where user_id = ?")
                .bind(user_id)
                .fetch_all(&mut *connection)
                .await?;

        let configs: PipeConfigs = PipeConfigs { configs: rows };
        Ok(configs)
    }
}

#[derive(Debug)]
pub struct App {
    database: Database,
}

#[derive(RustEmbed)]
#[folder = "../console/out/"]
pub struct Assets;

impl App {
    async fn delete_config(&self, id: u64, user_id: &str) -> Result<(), error::Error> {
        self.database.delete_config(id, user_id).await?;
        Ok(())
    }

    fn validate_configs(&self, new_configs: &PipeConfigs) -> Result<(), error::Error> {
        for config in new_configs.configs.iter() {
            let pipe: Vec<serde_json::Value> = serde_json::from_value(config.pipe.clone())?;
            for p in pipe {
                // all sections need a name because we use that to identify which type of section to construct
                let name = p
                    .get("name")
                    .ok_or(error::Error::Str("section is missing 'name' field"))?;
                if name != "mycelial_server_source" && name != "mycelial_server_destination" {
                    let _client = p
                        .get("client")
                        .ok_or(error::Error::Str("section is missing 'client' field"))?;
                }
                // Should we try to construct the section here to make sure it's valid? Can we?
            }
        }
        Ok(())
    }

    /// Set pipe configs
    async fn set_configs(
        &self,
        new_configs: &PipeConfigs,
        user_id: &str,
    ) -> Result<Vec<u64>, error::Error> {
        let mut inserted_ids = Vec::new();
        for config in new_configs.configs.iter() {
            let id = self
                .database
                .insert_config(
                    &config.pipe,
                    config.workspace_id.try_into().unwrap(),
                    user_id,
                )
                .await?;
            inserted_ids.push(id);
        }
        Ok(inserted_ids)
    }

    async fn update_configs(
        &self,
        configs: PipeConfigs,
        user_id: &str,
    ) -> Result<(), error::Error> {
        for config in configs.configs {
            self.database
                .update_config(config.id, &config.pipe, user_id)
                .await?
        }
        Ok(())
    }

    async fn update_config(
        &self,
        config: PipeConfig,
        user_id: &str,
    ) -> Result<PipeConfig, error::Error> {
        self.database
            .update_config(config.id, &config.pipe, user_id)
            .await?;
        Ok(config)
    }

    async fn get_config(&self, id: u64, user_id: &str) -> Result<PipeConfig, error::Error> {
        self.database.get_config(id, user_id).await
    }

    async fn get_workspaces(&self, user_id: &str) -> Result<Vec<Workspace>, error::Error> {
        self.database.get_workspaces(user_id).await
    }

    async fn create_workspace(
        &self,
        workspace: Workspace,
        user_id: &str,
    ) -> Result<Workspace, error::Error> {
        self.database.create_workspace(workspace, user_id).await
    }

    async fn get_workspace(&self, id: u64, user_id: &str) -> Result<Workspace, error::Error> {
        self.database.get_workspace(id, user_id).await
    }

    async fn update_workspace(
        &self,
        workspace: Workspace,
        user_id: &str,
    ) -> Result<Workspace, error::Error> {
        self.database.update_workspace(workspace, user_id).await
    }

    async fn delete_workspace(&self, id: u64, user_id: &str) -> Result<(), error::Error> {
        self.database.delete_workspace(id, user_id).await
    }

    async fn get_configs(&self, user_id: &str) -> Result<PipeConfigs, error::Error> {
        self.database.get_configs(user_id).await
    }

    async fn get_user_id_for_daemon_token(&self, token: &str) -> Result<String, error::Error> {
        let mut connection = self.database.get_connection().await;
        let user_id: String =
            sqlx::query("SELECT user_id FROM user_daemon_tokens WHERE daemon_token = ?")
                .bind(token)
                .fetch_one(&mut *connection)
                .await
                .map(|row| row.get(0))?;
        Ok(user_id)
    }

    async fn get_user_daemon_token(
        &self,
        user_id: &str,
    ) -> Result<impl IntoResponse, error::Error> {
        let mut connection = self.database.get_connection().await;
        let daemon_token =
            sqlx::query("SELECT daemon_token FROM user_daemon_tokens WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(&mut *connection)
                .await?
                .get::<String, _>(0);
        Ok(daemon_token)
    }

    async fn rotate_user_daemon_token(
        &self,
        user_id: &str,
    ) -> Result<impl IntoResponse, error::Error> {
        // create a new token
        let token = Uuid::new_v4().to_string();
        let mut connection = self.database.get_connection().await;
        // todo: Should this schema have "deleted_at" and then we only insert rows?
        sqlx::query(
            "INSERT OR REPLACE INTO user_daemon_tokens (user_id, daemon_token) VALUES (?, ?)",
        )
        .bind(user_id)
        .bind(token.clone())
        .execute(&mut *connection)
        .await?;
        Ok(token)
    }

    // should probably move this to db fn
    async fn validate_client_id_and_secret(
        &self,
        client_id: &str,
        secret: &str,
    ) -> Result<String, error::Error> {
        let mut connection = self.database.get_connection().await;
        let (user_id, client_secret_hash): (String, String) = sqlx::query(
            "SELECT user_id, client_secret_hash FROM clients WHERE unique_client_id = ?",
        )
        .bind(client_id)
        .fetch_one(&mut *connection)
        .await
        .map(|row| (row.get(0), row.get(1)))?;
        bcrypt::verify(secret, client_secret_hash.as_str())
            .map_err(|_| error::Error::Str("bcrypt error"))
            .and_then(|v| {
                if v {
                    Ok(user_id)
                } else {
                    Err(error::Error::Str("invalid client secret"))
                }
            })
    }
}

impl App {
    pub async fn new(db_path: impl AsRef<str>) -> anyhow::Result<Self> {
        let db_path: &str = db_path.as_ref();
        if let Some(parent) = Path::new(db_path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        };
        let database = Database::new(db_path).await?;
        Ok(Self { database })
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
    pretty_env_logger::init();

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
        .route("/api/pipe", post(post_pipe_config).put(put_pipe_configs))
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
        .route(
            "/api/daemon_token",
            post(rotate_user_daemon_token).get(get_user_daemon_token),
        )
        .route("/api/clients", get(get_clients));
    // check to see if auth feature is turned on.
    if cfg!(feature = "require_auth") {
        protected_api = protected_api.layer(middleware::from_fn(user_auth));
    }

    let daemon_basic_api = Router::new()
        .route("/api/client", post(provision_client))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            validate_client_basic_auth,
        ));

    // daemon uses its client_id and client_secret to auth, regardless of whether user auth is turned on
    let daemon_protected_api = Router::new()
        .route("/api/pipe", get(get_pipe_configs))
        .layer(middleware::from_fn_with_state(state.clone(), daemon_auth));

    // ingestion api is "security by obscurity" for now, and relies on the topic being secret
    let ingestion_api = Router::new()
        .route("/ingestion/:topic", post(ingestion))
        .route("/ingestion/:topic/:offset", get(get_message));

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
