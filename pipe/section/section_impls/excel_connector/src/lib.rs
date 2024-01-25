use calamine::DataType as ExcelDataType;
use section::message::{Ack, Chunk, Column, DataFrame, DataType, Message, Value};
use std::sync::Arc;

pub mod source;

#[derive(Debug)]
pub(crate) struct Sheet {
    pub name: Arc<str>,
    pub columns: Arc<[TableColumn]>,
}

#[derive(Debug)]
pub(crate) struct TableColumn {
    name: Arc<str>,
    data_type: DataType,
}

#[derive(Debug)]
pub(crate) struct ExcelPayload {
    columns: Arc<[TableColumn]>,
    values: Vec<Vec<Value>>,
}

impl DataFrame for ExcelPayload {
    fn columns(&self) -> Vec<Column<'_>> {
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

pub struct ExcelMessage {
    origin: Arc<str>,
    payload: Option<Box<dyn DataFrame>>,
    ack: Option<Ack>,
}

impl ExcelMessage {
    fn new(origin: Arc<str>, payload: ExcelPayload, ack: Option<Ack>) -> Self {
        Self {
            origin,
            payload: Some(Box::new(payload)),
            ack,
        }
    }
}

impl Message for ExcelMessage {
    fn origin(&self) -> &str {
        self.origin.as_ref()
    }

    fn next(&mut self) -> section::message::Next<'_> {
        let v = self.payload.take().map(Chunk::DataFrame);
        Box::pin(async move { Ok(v) })
    }

    fn ack(&mut self) -> section::message::Ack {
        self.ack.take().unwrap_or(Box::pin(async {}))
    }
}

impl std::fmt::Debug for ExcelMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExcelMessage")
            .field("origin", &self.origin)
            .field("payload", &self.payload)
            .finish()
    }
}

pub(crate) struct ExcelDataTypeWrapper<'a> {
    value: &'a ExcelDataType,
    stringify: bool,
}

impl<'a> ExcelDataTypeWrapper<'a> {
    pub fn new(value: &'a ExcelDataType, stringify: bool) -> Self {
        Self { value, stringify }
    }
}

impl From<ExcelDataTypeWrapper<'_>> for Value {
    fn from(val: ExcelDataTypeWrapper<'_>) -> Self {
        match val.value {
            value if val.stringify => Value::from(value.to_string()),
            ExcelDataType::Int(v) => Value::I64(*v),
            ExcelDataType::Float(f) => Value::F64(*f),
            ExcelDataType::String(s) => Value::from(s.to_string()),
            ExcelDataType::Bool(b) => Value::Bool(*b),
            ExcelDataType::DateTime(_) => {
                // TODO: do we have a datetime format rather than string?
                // FIXME: unwrap
                Value::from(
                    val.value
                        .as_datetime()
                        .unwrap()
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string(),
                )
            }
            ExcelDataType::Duration(f) => Value::F64(*f),
            ExcelDataType::DateTimeIso(d) => Value::Str(d.as_str().into()),
            ExcelDataType::DurationIso(d) => Value::Str(d.as_str().into()),
            ExcelDataType::Error(e) => Value::Str(e.to_string().into()),
            ExcelDataType::Empty => Value::Null,
        }
    }
}
