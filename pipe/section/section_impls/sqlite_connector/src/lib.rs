use section::message::{Ack, Chunk, Column, DataFrame, DataType, Message, Value};

//pub mod destination;
pub mod source;

#[derive(Debug, PartialEq)]
pub struct SqlitePayload {
    /// column names
    pub columns: Vec<String>,

    /// column types
    pub column_types: Vec<DataType>,

    /// values
    pub values: Vec<Vec<Value>>,
}

impl DataFrame for SqlitePayload {
    fn columns(&self) -> Vec<section::message::Column<'_>> {
        self.columns
            .iter()
            .zip(self.values.iter())
            .map(|(col_name, column)| {
                Column::new(col_name.as_str(), Box::new(column.iter().map(Into::into)))
            })
            .collect()
    }
}

pub struct SqliteMessage {
    origin: String,
    payload: Option<Box<dyn DataFrame>>,
    ack: Option<Ack>,
}

impl SqliteMessage {
    fn new(origin: impl Into<String>, payload: SqlitePayload, ack: Option<Ack>) -> Self {
        Self {
            origin: origin.into(),
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
        self.origin.as_str()
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
