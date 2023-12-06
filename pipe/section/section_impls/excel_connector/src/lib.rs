use chrono::NaiveDateTime;
use section::message::{Ack, Chunk, Column, DataFrame, DataType, Message, Value};
use std::{fmt::Display, sync::Arc};

pub mod source;

type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;
// pub type Message = _Message<ExcelPayload>;

#[derive(Debug, Clone, PartialEq)]
pub struct ExcelPayload {
    /// column names
    pub columns: Arc<[String]>,

    /// column types
    pub column_types: Arc<[ColumnType]>,

    /// values
    pub values: Vec<Vec<Value>>,

    /// offset
    pub offset: i64,
}

pub(crate) struct TableColumn {
    name: Arc<str>,
    data_type: DataType,
    nullable: bool,
}
pub(crate) struct NewExcelPayload {
    columns: Arc<[TableColumn]>,
    values: Vec<Vec<Value>>,
}

impl DataFrame for NewExcelPayload {
    fn columns(&self) -> Vec<Column<'_>> {
        self.columns
            .iter()
            .zip(self.values.iter())
            .map(|(col, column)| {
                Column::new(col.name.as_ref(), Box::new(column.iter().map(Into::into)))
            })
            .collect()
    }
}

pub struct ExcelMessage {
    origin: Arc<str>,
    payload: Option<Box<dyn DataFrame>>,
    ack: Option<Ack>,
}

impl ExcelMessage {
    fn new(origin: Arc<str>, payload: NewExcelPayload, ack: Option<Ack>) -> Self {
        Self {
            origin,
            payload: Some(Box::new(payload)),
            ack,
        }
    }
}

impl Message for ExcelMessage {
    fn origin(&self) -> &str {
        &self.origin.as_ref()
    }

    fn next(&mut self) -> section::message::Next<'_> {
        let v = self.payload.take().map(Chunk::DataFrame);
        Box::pin(async move { Ok(v) })
    }

    fn ack(&mut self) -> section::message::Ack {
        self.ack.take().unwrap_or(Box::pin(async {}))
    }
}

// FIXME: numeric?
// redo whole value enum?
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    Int(i64),
    Text(String),
    Blob(Vec<u8>),
    Real(f64),
    Bool(bool),
    DateTime(NaiveDateTime),
    #[default]
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColumnType {
    Int,
    Text,
    Blob,
    Real,
    Numeric,
    Bool,
    DateTime,
    Duration,
    DateTimeIso,
    DurationIso,
}

impl Display for ColumnType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ty = match self {
            ColumnType::Int => "INTEGER",
            ColumnType::Text => "TEXT",
            ColumnType::Blob => "BLOB",
            ColumnType::Real => "DOUBLE",
            ColumnType::Numeric => "NUMERIC",
            ColumnType::Bool => "BOOL",
            ColumnType::DateTime => "DATETIME",
            ColumnType::Duration => "DURATION",
            ColumnType::DateTimeIso => "DATETIMEISO",
            ColumnType::DurationIso => "DURATIONISO",
        };
        write!(f, "{}", ty)
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
