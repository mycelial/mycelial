//! Message
//!
//! Message is a data struct which used to communicate between sections in the pipe.
use arrow::record_batch::RecordBatch as _RecordBatch;
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
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

pub type FnAck = Box<dyn FnOnce() + Send + Sync + 'static>;

pub struct Message {
    pub origin: String,
    pub payload: RecordBatch,
    pub ack: Option<FnAck>,
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Message")
            .field("origin", &self.origin)
            .field("payload", &self.payload)
            .finish()
    }
}

impl Message {
    pub fn new(
        origin: impl Into<String>,
        payload: impl Into<RecordBatch>,
        ack: Option<FnAck>,
    ) -> Self {
        Self {
            origin: origin.into(),
            payload: payload.into(),
            ack,
        }
    }

    pub fn ack(&mut self) {
        if let Some(ack) = self.ack.take() {
            ack()
        }
    }
}
