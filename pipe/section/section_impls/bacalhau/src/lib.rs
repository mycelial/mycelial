pub mod destination;
pub mod source;

type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;

use std::sync::Arc;

use arrow::{
    array::{ArrayRef, StringArray},
    datatypes::{DataType, Field, Schema},
    error::ArrowError,
    record_batch::RecordBatch,
};
use section::Message as _Message;
use serde::{Deserialize, Serialize};
pub type Message = _Message<BacalhauPayload>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BacalhauPayload {
    pub id: String,
    pub key: String,
    pub message: String,
}

impl BacalhauPayload {
    fn new(id: impl Into<String>, key: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            key: key.into(),
            message: message.into(),
        }
    }
}

impl TryFrom<RecordBatch> for BacalhauPayload {
    type Error = StdError;

    fn try_from(batch: RecordBatch) -> Result<Self, Self::Error> {
        Self::try_from(&batch)
    }
}

impl TryFrom<&RecordBatch> for BacalhauPayload {
    type Error = StdError;

    fn try_from(batch: &RecordBatch) -> Result<Self, Self::Error> {
        // fudge: to json and pull 'key' and 'message'
        println!("B: {:?}", &batch);
        let json_rows = arrow_json::writer::record_batches_to_json_rows(&[&batch]).unwrap();
        let row = json_rows.first().unwrap();

        let Some(id) = row.get("id").map(|v| v.as_i64()) else {
            return Err("missing 'id'".into());
        };
        let Some(key) = row.get("key").map(|v| v.as_str()) else {
            return Err("missing 'key'".into());
        };
        let Some(message) = row.get("message").map(|v| v.as_str()) else {
            return Err("missing 'message'".into());
        };

        Ok(Self {
            id: id.unwrap().to_string(),
            key: key.unwrap().into(),
            message: message.unwrap().into(),
        })
    }
}

impl TryInto<RecordBatch> for &BacalhauPayload {
    // FIXME: proper conv error type
    type Error = ArrowError;

    fn try_into(self) -> Result<RecordBatch, Self::Error> {
        let schema: Arc<Schema> = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, true),
            Field::new("key", DataType::Utf8, true),
            Field::new("message", DataType::Utf8, true),
        ]));
        let columns: Vec<ArrayRef> = vec![
            Arc::new(StringArray::from(vec![self.id.clone()])),
            Arc::new(StringArray::from(vec![self.key.clone()])),
            Arc::new(StringArray::from(vec![self.message.clone()])),
        ];
        let r = RecordBatch::try_new(schema, columns)?;
        Ok(r)
    }
}

impl TryInto<RecordBatch> for BacalhauPayload {
    // FIXME: proper conv error type
    type Error = ArrowError;

    fn try_into(self) -> Result<RecordBatch, Self::Error> {
        (&self).try_into()
    }
}
