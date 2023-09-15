use std::{sync::Arc, fmt::Display};
use section::Message as _Message;

pub mod source;
//pub mod destination;

pub type Message = _Message<SqlitePayload>;

#[derive(Debug)]
pub struct SqlitePayload {
    /// column names
    pub columns: Arc<[String]>,

    /// column types
    pub column_types: Arc<[ColumnType]>,

    /// values
    pub values: Vec<Vec<Value>>,

    /// offset
    pub offset: i64
}


// FIXME: numeric?
// redo whole value enum?
#[derive(Debug)]
pub enum Value {
    Int(i64),
    Text(String),
    Blob(Vec<u8>),
    Real(f64),
    Null,
}

#[derive(Debug, Clone, Copy)]
pub enum ColumnType {
    Int,
    Text,
    Blob,
    Real,
    Numeric,
}

impl Display for ColumnType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ty = match self {
            ColumnType::Int => "INTEGER",
            ColumnType::Text => "TEXT",
            ColumnType::Blob => "BLOB",
            ColumnType::Real => "DOUBLE",
            ColumnType::Numeric => "NUMERIC",
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
                _   => None,
            };
            maybe_char.into_iter().chain([char])
        }).collect::<String>()
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
    format!(
        "CREATE TABLE IF NOT EXISTS \"{name}\" ({columns})",
    )
}
