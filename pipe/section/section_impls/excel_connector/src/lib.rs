use chrono::NaiveDateTime;
use section::Message as _Message;
use std::{fmt::Display, sync::Arc};

pub mod source;

type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type Message = _Message<ExcelPayload>;

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