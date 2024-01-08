use std::sync::Arc;

use section::{
    message::{Ack, Chunk, Column, DataFrame, DataType, Message, Value},
    SectionError,
};

pub mod destination;
pub mod source;

#[derive(Debug)]
#[allow(unused)]
pub(crate) struct Table {
    name: Arc<str>,
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
}

#[derive(Debug, Clone)]
pub struct MysqlPayload {
    /// column names
    columns: Arc<[TableColumn]>,

    /// values
    values: Vec<Vec<Value>>,
}

impl DataFrame for MysqlPayload {
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

pub struct MysqlMessage {
    origin: Arc<str>,
    payload: Option<Box<dyn DataFrame>>,
    ack: Option<Ack>,
}

impl MysqlMessage {
    fn new(origin: Arc<str>, payload: impl DataFrame, ack: Option<Ack>) -> Self {
        Self {
            origin,
            payload: Some(Box::new(payload)),
            ack,
        }
    }
}

impl std::fmt::Debug for MysqlMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MysqlMessage")
            .field("origin", &self.origin)
            .field("payload", &self.payload)
            .finish()
    }
}

impl Message for MysqlMessage {
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
pub fn generate_schema(table_name: &str, df: &dyn DataFrame) -> Result<String, SectionError> {
    let name = escape_table_name(table_name);
    let columns = df
        .columns()
        .iter()
        .map(|col| {
            let cdt = col.data_type();
            let dtype = match col.data_type() {
                DataType::I8 | DataType::I16 => "SMALLINT",
                DataType::I32 => "INTEGER",
                DataType::I64 => "BIGINT",
                DataType::F32 => "REAL",
                DataType::F64 => "DOUBLE",
                DataType::Decimal => "NUMERIC",
                DataType::RawJson => "JSON",
                DataType::Str => "TEXT",
                DataType::Bin => "BLOB",
                DataType::Time => "TIME",
                DataType::Date => "DATE",
                DataType::TimeStamp => "TIMESTAMP",
                DataType::Uuid => "UUID",
                DataType::Bool => "TINYINT",
                DataType::U8 => "SMALLINT",
                DataType::U16 => "INT",
                DataType::U32 => "BIGINT",
                DataType::U64 => "DOUBLE",
                DataType::Any => "TEXT", // I don't think this is fully valid but it kinda works?
                v => return Err(format!("unsupported type {v:?}")),
            };
            Ok(format!("{} {}", col.name(), dtype))
        })
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    Ok(format!("CREATE TABLE IF NOT EXISTS `{name}` ({columns})",))
}
