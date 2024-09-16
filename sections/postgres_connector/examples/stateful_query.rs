use std::time::Instant;

use section::{
    dummy::DummySectionChannel,
    futures::{self, SinkExt, StreamExt},
    message::{Chunk, ValueView},
    section::Section,
    SectionError,
};
use stub::Stub;

#[tokio::main]
async fn main() -> Result<(), SectionError> {
    tracing_subscriber::fmt::init();

    let pg_src = postgres_connector::PostgresSource::new(
        "localhost",
        5432,
        "user",
        "password",
        "postgres",
        "public",
        5,
        "select * from test where id > $id::i64 order by id limit 10000",
    );
    let (tx, mut rx) = futures::channel::mpsc::channel(1);
    let tx = tx.sink_map_err(|_| "send error".into());
    let handle = tokio::spawn(pg_src.start(Stub::<()>::new(), tx, DummySectionChannel::new()));
    let mut start = Instant::now();
    while let Some(mut msg) = rx.next().await {
        let mut count = 0;
        let mut min = i64::MAX;
        let mut max = 0;
        while let Ok(Some(chunk)) = msg.next().await {
            match chunk {
                Chunk::DataFrame(df) => {
                    let (new_min, new_max, new_count) = df
                        .columns()
                        .into_iter()
                        .filter(|col| col.name() == "id")
                        .fold(
                            (min, max, count),
                            |(mut min, mut max, mut count), mut col| {
                                for next in col.by_ref() {
                                    match next {
                                        ValueView::I64(val) => {
                                            count += 1;
                                            min = min.min(val);
                                            max = max.max(val);
                                        }
                                        _ => panic!("unexpected value: {next}",),
                                    };
                                }
                                (min, max, count)
                            },
                        );
                    count = new_count;
                    min = new_min;
                    max = new_max;
                }
                _ => panic!("unexpected: {:?}", chunk),
            };
        }
        tracing::info!(
            "{} in {}ms min_id: {}, max_id: {}",
            count,
            start.elapsed().as_millis(),
            min,
            max
        );
        start = Instant::now();
    }
    tracing::error!("{:?}", handle.await?);
    Ok(())
}
