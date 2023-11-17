use std::time::Duration;

use futures::{SinkExt, StreamExt};
use section::{Section, dummy::DummySectionChannel};
use stub::Stub;


#[tokio::main]
async fn main() {
    let pg_src = postgres_connector::source::Postgres::new(
        "postgres://user:password@localhost:5432/test",
        "public",
        &["*"],
        Duration::from_secs(5),
    );
    let (tx, mut rx) = futures::channel::mpsc::channel(1);
    let tx = tx.sink_map_err(|_| "send error".into());
    let handle = tokio::spawn(pg_src.start(Stub::<()>::new(), tx, DummySectionChannel::new()));
    while let Some(msg) = rx.next().await {
        println!("got message: {:?}", msg);
    }
    println!("{:?}", handle.await);
}
