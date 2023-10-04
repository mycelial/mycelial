//! Message
//!
//! Message is a data struct which used to communicate between sections in the pipe.
use arrow::record_batch::RecordBatch as _RecordBatch;
use section::Message as _Message;
use std::ops::{Deref, DerefMut};

#[derive(Debug, PartialEq)]
#[repr(transparent)]
pub struct RecordBatch(pub _RecordBatch);

impl From<RecordBatch> for _RecordBatch {
    fn from(val: RecordBatch) -> Self {
        val.0
    }
}

impl From<_RecordBatch> for RecordBatch {
    fn from(val: _RecordBatch) -> Self {
        Self(val)
    }
}

impl Deref for RecordBatch {
    type Target = _RecordBatch;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RecordBatch {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub type Message = _Message<RecordBatch>;
