//! Mycelial server
use arrow::ipc::reader::StreamReader;
use axum::{
    extract::BodyStream,
    headers::{authorization::Basic, Authorization},
    http::{Method, Request, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router, Server, TypedHeader, Json,
};
use chrono::Utc;
use clap::Parser;
use futures::StreamExt;
use serde::Serialize;
use serde_json::json;
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

#[derive(Serialize)]
struct Configs {
    configs: Vec<RawConfig>
}

#[derive(Serialize)]
struct RawConfig {
    id: u64,
    raw_config: String,
}

async fn get_pipe_configs() -> Json<Configs>{
    let configs = Configs {
        configs: vec![
            RawConfig{
                id: 1, 
                raw_config: serde_json::to_string(
                    &json!({
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
                    })
                ).unwrap()
            }
        ]
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = CLI::try_parse()?;
    let router = Router::new()
        .route("/ingestion", post(ingestion))
        .route("/pipe/configs", get(get_pipe_configs))
        .layer(middleware::from_fn(log));

    let addr: SocketAddr = cli.listen_addr.as_str().parse()?;
    Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;
    Ok(())
}
