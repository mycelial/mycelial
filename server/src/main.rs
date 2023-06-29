//! Mycelial server
use arrow::ipc::reader::StreamReader;
use axum::{extract::BodyStream, response::IntoResponse, routing::post, Router, Server};
use clap::Parser;
use futures::StreamExt;
use std::net::SocketAddr;

#[derive(Parser)]
struct CLI {
    #[clap(short, long, env = "LISTEN_ADDR", default_value = "0.0.0.0:8080")]
    listen_addr: String,
}

async fn ingestion(mut body: BodyStream) -> impl IntoResponse {
    let mut buf = vec![];
    while let Some(chunk) = body.next().await {
        buf.append(&mut chunk.unwrap().to_vec());
    };
    let reader = StreamReader::try_new(buf.as_slice(), None).unwrap();
    for record_batch in reader {
        println!("got new record batch:\n{:?}", record_batch);
    }
    ""
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = CLI::try_parse()?;
    let router = Router::new().route("/ingestion", post(ingestion));

    let addr: SocketAddr = cli.listen_addr.as_str().parse()?;
    Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;
    Ok(())
}
