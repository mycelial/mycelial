use section::Message as _Message;
use std::{fmt::Display, sync::Arc};

pub mod destination;
pub mod source;

type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type Message = _Message<PostgresPayload>;

#[derive(Debug, Clone, PartialEq)]
pub struct PostgresPayload {
    /// column names
    pub columns: Arc<[String]>,

    /// column types
    pub column_types: Arc<[ColumnType]>,

    /// values
    pub values: Vec<Vec<Value>>,

    /// offset
    pub offset: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Text(String),
    Blob(Vec<u8>),
    Bool(bool),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColumnType {
    I16,
    I32,
    I64,
    F32,
    F64,
    Text,
    Blob,
    Numeric,
    Bool,
}

impl Display for ColumnType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ty = match self {
            ColumnType::I16 => "SMALLINT",
            ColumnType::I32 => "INTEGER",
            ColumnType::I64 => "BIGINT",
            ColumnType::F32 => "REAL",
            ColumnType::F64 => "DOUBLE PRECISION",
            ColumnType::Text => "TEXT",
            ColumnType::Blob => "BLOB",
            ColumnType::Numeric => "NUMERIC",
            ColumnType::Bool => "BOOLEAN",
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

/// generate create table command for provided record batch
pub fn generate_schema(message: &Message) -> String {
    let name = escape_table_name(message.origin.as_str());
    let payload = &message.payload;
    let columns = payload
        .columns
        .iter()
        .zip(payload.column_types.iter())
        .map(|(name, ty)| format!("{name} {ty}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("CREATE TABLE IF NOT EXISTS \"{name}\" ({columns})",)
}
