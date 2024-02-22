use std::str::FromStr;

use section::futures::{SinkExt, StreamExt};
use section::message::{Chunk, Value};

use section::section::Section as _;
use section::{dummy::*, SectionMessage};
use sqlite_connector::source;
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, SqliteConnection};
use stub::Stub;
use tempfile::NamedTempFile;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;

type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;

async fn init_sqlite(path: &str) -> Result<SqliteConnection, StdError> {
    let mut conn = SqliteConnectOptions::from_str(path)
        .unwrap()
        .create_if_missing(true)
        .connect()
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS test (id INT PRIMARY KEY NOT NULL, text TEXT, bin BLOB, float DOUBLE)",
    )
    .execute(&mut conn)
    .await?;

    sqlx::query(
        "INSERT OR IGNORE INTO test VALUES(1, 'foo', 'foo', 1), (2, 'bar', NULL, 0.2), ('this', 'is', 'not', 'strict')",
    )
        .execute(&mut conn)
        .await?;
    sqlx::query("INSERT OR IGNORE INTO test VALUES('', 'bin', 'incoming', ?)")
        .bind(vec![b'b', b'i', b'n'].as_slice())
        .execute(&mut conn)
        .await?;
    Ok(conn)
}

pub fn channel<T>(buf_size: usize) -> (PollSender<T>, ReceiverStream<T>)
where
    T: Send + 'static,
{
    let (tx, rx): (Sender<T>, Receiver<T>) = tokio::sync::mpsc::channel(buf_size);
    (PollSender::new(tx), ReceiverStream::new(rx))
}

struct DropFile {
    path: String,
}

impl Drop for DropFile {
    fn drop(&mut self) {
        std::fs::remove_file(self.path.as_str()).ok();
    }
}

#[tokio::test]
async fn source_stream() -> Result<(), StdError> {
    let db_path = NamedTempFile::new()?.path().to_string_lossy().to_string();
    let _conn = init_sqlite(db_path.as_str()).await?;

    let section_chan = DummySectionChannel::new();

    let sqlite_source = source::Sqlite::new(db_path.as_str(), "test", "SELECT * FROM test");
    let (output, mut rx) = channel(1);
    let output = output.sink_map_err(|_| "chan closed".into());
    let input = Stub::<SectionMessage, StdError>::new();

    // cleanup file on exit
    let _drop_file = DropFile { path: db_path };

    let section = sqlite_source.start(input, output, section_chan);
    let handle = tokio::spawn(section);

    let mut out = rx.next().await.unwrap();
    assert_eq!(out.origin(), "test");

    let chunk = out.next().await;
    assert!(matches!(chunk, Ok(Some(_))));
    assert!(matches!(out.next().await, Ok(None)));

    let df = match chunk.unwrap().unwrap() {
        Chunk::DataFrame(df) => df,
        other => panic!("unexpected chunk type: {:?}", other),
    };

    let columns = df.columns();
    assert_eq!(
        vec!["id", "text", "bin", "float"],
        columns.iter().map(|col| col.name()).collect::<Vec<_>>()
    );

    let payload = columns
        .into_iter()
        .map(|col| col.collect::<Vec<_>>())
        .collect::<Vec<_>>();

    assert_eq!(
        payload,
        vec![
            vec![
                Value::I64(1),
                Value::I64(2),
                Value::Str("this".into()),
                Value::Str("".into())
            ],
            vec![
                Value::Str("foo".into()),
                Value::Str("bar".into()),
                Value::Str("is".into()),
                Value::Str("bin".into()),
            ],
            vec![
                Value::Str("foo".into()),
                Value::Null,
                Value::Str("not".into()),
                Value::Str("incoming".into()),
            ],
            vec![
                Value::F64(1.0),
                Value::F64(0.2),
                Value::Str("strict".into()),
                Value::Bin("bin".as_bytes().into()),
            ],
        ]
    );
    handle.abort();
    Ok(())
}
