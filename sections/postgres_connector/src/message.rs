use std::sync::Arc;

use section::{
    message::{Ack, Chunk, Column, DataFrame, DataType, Message, Value},
    SectionError,
};
use tokio::sync::mpsc::Receiver;

#[allow(unused)]
pub(crate) struct Table {
    name: Arc<str>,
    columns: Arc<[PostgresColumn]>,
    query: String,
    offset: i64,
    limit: i64,
}

#[derive(Debug)]
#[allow(unused)]
pub(crate) struct PostgresColumn {
    name: String,
    data_type: DataType,
}

impl PostgresColumn {
    pub fn new(name: impl Into<String>, data_type: impl Into<DataType>) -> Self {
        Self {
            name: name.into(),
            data_type: data_type.into(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct PostgresPayload {
    pub columns: Vec<PostgresColumn>,
    pub values: Vec<Vec<Value>>,
}

impl DataFrame for PostgresPayload {
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

pub struct PostgresMessage {
    origin: Arc<str>,
    stream: Receiver<Option<Chunk>>,
    ack: Option<Ack>,
}

impl PostgresMessage {
    pub fn new(origin: Arc<str>, stream: Receiver<Option<Chunk>>, ack: Option<Ack>) -> Self {
        Self {
            origin,
            stream,
            ack,
        }
    }
}

impl std::fmt::Debug for PostgresMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresMessage")
            .field("origin", &self.origin)
            .finish()
    }
}

impl Message for PostgresMessage {
    fn origin(&self) -> &str {
        self.origin.as_ref()
    }

    fn next(&mut self) -> section::message::Next<'_> {
        Box::pin(async move {
            match self.stream.recv().await {
                Some(Some(df)) => Ok(Some(df)),
                Some(None) => Ok(None),
                None => Err("closed")?,
            }
        })
    }

    fn ack(&mut self) -> Ack {
        self.ack.take().unwrap_or(Box::pin(async {}))
    }
}

/// Escape value
pub fn escape(name: impl AsRef<str>) -> String {
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
pub fn generate_schema(
    schema: &str,
    table_name: &str,
    df: &dyn DataFrame,
) -> Result<String, SectionError> {
    let name = escape(table_name);
    let columns = df
        .columns()
        .iter()
        .map(|col| {
            let dtype = match col.data_type() {
                DataType::I8 | DataType::I16 => "SMALLINT",
                DataType::I32 => "INTEGER",
                DataType::I64 => "BIGINT",
                DataType::F32 => "REAL",
                DataType::F64 => "DOUBLE PRECISION",
                DataType::Decimal => "NUMERIC",
                DataType::RawJson => "JSON",
                DataType::Str => "TEXT",
                DataType::Bin => "BYTEA",
                DataType::Time(_) => "TIME",
                DataType::Date(_) => "DATE",
                DataType::TimeStamp(_) => "TIMESTAMP",
                DataType::TimeStampUTC(_) => "TIMESTAMPTZ",
                DataType::Uuid => "UUID",
                v => return Err(format!("unsupported type {v:?}")),
            };
            Ok(format!("{} {}", col.name(), dtype))
        })
        .collect::<Result<Vec<_>, _>>()?
        .join(",");
    Ok(format!(
        "CREATE TABLE IF NOT EXISTS \"{schema}\".\"{name}\" ({columns})"
    ))
}
