use std::sync::Arc;

use section::{
    message::{Ack, Chunk, Column, DataFrame, DataType, Message, Value},
    SectionError,
};
use tokio::sync::mpsc::Receiver;

pub mod destination;
pub mod source;

#[derive(Debug)]
#[allow(unused)]
pub(crate) struct SqliteColumn {
    name: String,
    data_type: DataType,
}

#[derive(Debug)]
pub(crate) struct SqlitePayload {
    /// column names
    columns: Vec<SqliteColumn>,

    /// values
    values: Vec<Vec<Value>>,
}

impl DataFrame for SqlitePayload {
    fn columns(&self) -> Vec<section::message::Column<'_>> {
        self.columns
            .iter()
            .zip(self.values.iter())
            .map(|(col, column)| {
                Column::new(
                    col.name.as_ref(),
                    col.data_type,
                    Box::new(column.iter().map(Into::into)),
                )
            })
            .collect()
    }
}

pub struct SqliteMessage {
    origin: Arc<str>,
    stream: Receiver<Result<Chunk, SectionError>>,
    ack: Option<Ack>,
}

impl SqliteMessage {
    fn new(
        origin: Arc<str>,
        stream: Receiver<Result<Chunk, SectionError>>,
        ack: Option<Ack>,
    ) -> Self {
        Self {
            origin,
            stream,
            ack,
        }
    }
}

impl std::fmt::Debug for SqliteMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteMessage")
            .field("origin", &self.origin)
            .field("stream", &self.stream)
            .finish()
    }
}

impl Message for SqliteMessage {
    fn origin(&self) -> &str {
        self.origin.as_ref()
    }

    fn next(&mut self) -> section::message::Next<'_> {
        Box::pin(async move {
            match self.stream.recv().await {
                Some(Ok(df)) => Ok(Some(df)),
                Some(Err(e)) => Err(e),
                None => Ok(None),
            }
        })
    }

    fn ack(&mut self) -> Ack {
        self.ack.take().unwrap_or(Box::pin(async {}))
    }
}

/// Escape table name
pub fn escape_table_name(name: impl AsRef<str>) -> String {
    name.as_ref()
        .chars()
        .flat_map(|char| {
            let maybe_char = match char {
                '"' => Some('\\'),
                _ => None,
            };
            maybe_char.into_iter().chain([char])
        })
        .collect::<String>()
}

/// generate create table command for provided record batch
pub fn generate_schema(table_name: &str, df: &dyn DataFrame) -> String {
    let name = escape_table_name(table_name);
    let columns = generate_column_names(df);
    format!("CREATE TABLE IF NOT EXISTS \"{name}\" ({columns})",)
}

pub fn generate_column_names(df: &dyn DataFrame) -> String {
    df.columns()
        .iter()
        .map(|col| col.name())
        .map(|name| {
            // wrap the name in quotes
            // escape all quotes by replacing with a double quote
            format!("\"{}\"", name.replace('\"', "\"\""))
        })
        .collect::<Vec<_>>()
        .join(",")
}
