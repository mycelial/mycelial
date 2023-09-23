use std::str::FromStr;

use futures::{SinkExt, StreamExt};
use section::dummy::*;
use section::Section as _;
use sqlite::{source, ColumnType, Message, SqlitePayload, Value};
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, Connection, SqliteConnection};
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
        "CREATE TABLE IF NOT EXISTS test (id INT PRIMARY KEY, text TEXT, bin BLOB, float DOUBLE)",
    )
    .execute(&mut conn)
    .await?;

    sqlx::query(
        "INSERT OR IGNORE INTO test VALUES(1, 'foo', 'foo', 1), (2, 'bar', 'bar', 0.2), (3, '', '', 30.0), (4, NULL, NULL, 40.0)",
    )
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
async fn source() -> Result<(), StdError> {
    let db_path = NamedTempFile::new()?.path().to_string_lossy().to_string();
    let _conn = init_sqlite(db_path.as_str()).await?;

    let section_chan = DummySectionChannel::new();

    let sqlite_source = source::Sqlite::new(db_path.as_str(), &["*"]);
    let (output, mut rx) = channel(1);
    let output = output.sink_map_err(|_| "chan closed".into());
    let input = Stub::<Message, StdError>::new();

    // cleanup file on exit
    let _drop_file = DropFile { path: db_path };

    let section = sqlite_source.start(input, output, section_chan);
    let handle = tokio::spawn(section);

    let out = rx.next().await.unwrap();
    assert_eq!(out.origin, "test");
    assert_eq!(
        out.payload,
        SqlitePayload {
            columns: vec![
                "id".to_string(),
                "text".to_string(),
                "bin".to_string(),
                "float".to_string()
            ]
            .into(),
            column_types: vec![
                ColumnType::Int,
                ColumnType::Text,
                ColumnType::Blob,
                ColumnType::Real,
            ]
            .into(),
            values: vec![
                vec![Value::Int(1), Value::Int(2), Value::Int(3), Value::Int(4),],
                vec![
                    Value::Text("foo".to_string()),
                    Value::Text("bar".to_string()),
                    Value::Text("".to_string()),
                    Value::Null,
                ],
                vec![
                    Value::Blob(vec![102, 111, 111]),
                    Value::Blob(vec![98, 97, 114]),
                    Value::Blob(vec![]),
                    Value::Null,
                ],
                vec![
                    Value::Real(1.0),
                    Value::Real(0.2),
                    Value::Real(30.0),
                    Value::Real(40.0),
                ],
            ],
            offset: 4,
        }
    );
    handle.abort();
    Ok(())
}
