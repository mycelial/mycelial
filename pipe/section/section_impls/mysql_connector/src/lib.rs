use std::sync::Arc;

use section::{
    message::{Ack, Chunk, Column, DataFrame, DataType, Message, Value},
    SectionError,
};
use tokio::sync::mpsc::Receiver;

pub mod destination;
pub mod source;

#[derive(Debug)]
pub(crate) struct MysqlColumn {
    name: String,
    data_type: DataType,
}

impl MysqlColumn {
    fn new(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
        }
    }
}

#[derive(Debug)]
pub struct MysqlPayload {
    /// column names
    columns: Vec<MysqlColumn>,

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
    stream: Receiver<Result<Chunk, SectionError>>,
}

impl MysqlMessage {
    fn new(origin: Arc<str>, stream: Receiver<Result<Chunk, SectionError>>) -> Self {
        Self { origin, stream }
    }
}

impl std::fmt::Debug for MysqlMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MysqlMessage")
            .field("origin", &self.origin)
            .finish()
    }
}

impl Message for MysqlMessage {
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
        Box::pin(async {})
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
            let dtype = match col.data_type() {
                DataType::Bool => "TINYINT",
                DataType::I8 => "TINYINT",
                DataType::I16 => "SMALLINT",
                DataType::I32 => "INTEGER",
                DataType::I64 => "BIGINT",
                DataType::U8 => "TINYINT UNSIGNED",
                DataType::U16 => "SMALLINT UNSIGNED",
                DataType::U32 => "INT UNSIGNED",
                DataType::U64 => "BIGINT UNSIGNED",
                DataType::F32 => "REAL",
                DataType::F64 => "DOUBLE",
                // FIXME:
                // 65 total len with 10 being used by scale
                // in future we can have advanced section configuration for such values
                DataType::Decimal => "NUMERIC(55, 10)",
                DataType::RawJson => "JSON",
                DataType::Str => "TEXT",
                DataType::Bin => "BLOB",
                DataType::Time(_) => "TIME",
                DataType::Date(_) => "DATE",
                DataType::TimeStamp(_) => "DATETIME",
                DataType::TimeStampUTC(_) => "DATETIME",
                DataType::Uuid => "UUID",
                v => return Err(format!("failed to generate schema, unsupported type {v:?}")),
            };
            Ok(format!("{} {}", col.name(), dtype))
        })
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    Ok(format!("CREATE TABLE IF NOT EXISTS `{name}` ({columns})",))
}
