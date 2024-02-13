use std::time::Duration;

use section::{
    dummy::DummySectionChannel,
    futures::{self, SinkExt, StreamExt},
    message::Chunk,
    pretty_print::pretty_print,
    section::Section,
    SectionError,
};
use stub::Stub;

#[tokio::main]
async fn main() -> Result<(), SectionError> {
    let pg_src = postgres_connector::source::Postgres::new(
        "postgres://user:password@localhost:5432/postgres",
        "public",
        "select * from test",
        Duration::from_secs(5),
    );
    let (tx, mut rx) = futures::channel::mpsc::channel(1);
    let tx = tx.sink_map_err(|_| "send error".into());
    let handle = tokio::spawn(pg_src.start(Stub::<()>::new(), tx, DummySectionChannel::new()));
    while let Some(mut msg) = rx.next().await {
        while let Some(chunk) = msg.next().await? {
            match chunk {
                Chunk::DataFrame(df) => println!("{}", pretty_print(df.as_ref())),
                _ => panic!("unexpected: {:?}", chunk),
            };
        }
    }
    println!("{:?}", handle.await);
    Ok(())
}
