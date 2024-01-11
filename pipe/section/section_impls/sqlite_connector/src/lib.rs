use std::sync::Arc;

use section::message::{Ack, Chunk, Column, DataFrame, DataType, Message, Value};

pub mod destination;
pub mod source;

#[derive(Debug)]
#[allow(unused)]
pub(crate) struct Table {
    name: Arc<str>,
    strict: bool,
    without_rowid: bool,
    columns: Arc<[TableColumn]>,
    query: String,
    offset: i64,
    limit: i64,
}

#[derive(Debug)]
#[allow(unused)]
pub(crate) struct TableColumn {
    name: Arc<str>,
    data_type: DataType,
    nullable: bool,
}

#[derive(Debug)]
pub(crate) struct SqlitePayload {
    /// column names
    columns: Arc<[TableColumn]>,

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
    payload: Option<Box<dyn DataFrame>>,
    ack: Option<Ack>,
}

impl SqliteMessage {
    fn new(origin: Arc<str>, payload: SqlitePayload, ack: Option<Ack>) -> Self {
        Self {
            origin,
            payload: Some(Box::new(payload)),
            ack,
        }
    }
}

impl std::fmt::Debug for SqliteMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteMessage")
            .field("origin", &self.origin)
            .field("payload", &self.payload)
            .finish()
    }
}

impl Message for SqliteMessage {
    fn origin(&self) -> &str {
        self.origin.as_ref()
    }

    fn next(&mut self) -> section::message::Next<'_> {
        let v = self.payload.take().map(Chunk::DataFrame);
        Box::pin(async move { Ok(v) })
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
    let columns = df
        .columns()
        .iter()
        .map(|col| col.name())
        .collect::<Vec<_>>()
        .join(",");
    format!("CREATE TABLE IF NOT EXISTS \"{name}\" ({columns})",)
}
