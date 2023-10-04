pub mod destination;
pub mod source;

use std::{sync::Arc, vec};

use arrow::{
    array::{ArrayRef, StringArray},
    datatypes::{DataType, Field, Schema},
    error::ArrowError,
    record_batch::RecordBatch as _RecordBatch,
};
use section::Message as _Message;

use crate::message::{self, RecordBatch};

pub type Message = _Message<HelloWorldPayload>;

#[derive(Debug, Clone, PartialEq)]
pub struct HelloWorldPayload {
    /// message
    pub message: String,
}

impl TryInto<RecordBatch> for &HelloWorldPayload {
    // FIXME: proper conv error type
    type Error = ArrowError;

    fn try_into(self) -> Result<RecordBatch, Self::Error> {
        let schema: Arc<Schema> = Arc::new(Schema::new(vec![Field::new(
            "message",
            DataType::Utf8,
            true,
        )]));
        let columns: Vec<ArrayRef> = vec![Arc::new(StringArray::from(vec![self.message.clone()]))];
        let r = _RecordBatch::try_new(schema, columns)?;

        Ok(message::RecordBatch(r))
    }
}

impl TryInto<RecordBatch> for HelloWorldPayload {
    // FIXME: proper conv error type
    type Error = ArrowError;

    fn try_into(self) -> Result<RecordBatch, Self::Error> {
        (&self).try_into()
    }
}
