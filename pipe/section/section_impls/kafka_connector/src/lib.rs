// Kafka source section 
// CAUTION: ALPHA QUALITY CODE :) Use with caution.
use section::message::{Ack, Chunk, Column, DataFrame, Message, Value};
pub mod destination;

type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;

use rdkafka::message::{OwnedMessage, Timestamp};

pub struct KafkaMessage {

}

impl KafkaMessage {
    pub fn new() -> Self {
        Self {}
    }
}

impl std::fmt::Debug for KafkaMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KafkaMessage")
            // .field("origin", &self.origin)
            // .field("payload", &self.payload)
            .finish()
    }
}

impl Message for KafkaMessage {
    fn origin(&self) -> &str {
        unimplemented!()
    }

    fn next(&mut self) -> section::message::Next<'_> {
        unimplemented!()
    }
    
    fn ack(&mut self) -> section::message::Ack {
        unimplemented!()
    }
}



















// use crate::message::RecordBatch;

// #[derive(Debug)]
// #[repr(transparent)]
// pub struct OwnedMessageNewType(OwnedMessage);

// impl From<&RecordBatch> for OwnedMessageNewType {
//     fn from(arrow_rb: &RecordBatch) -> Self {
//         println!("arrow_rb: {:?}", arrow_rb);
//         let buf = Vec::new();
//         let mut writer = arrow::json::LineDelimitedWriter::new(buf);
//         // todo:  this couldn't handle a "binary" column type
//         // `JsonError("data type Binary not supported in nested map for json writer")`
//         writer.write(arrow_rb).unwrap();
//         writer.finish().unwrap();
//         let buf = writer.into_inner();

//         // TODO: Basic message works for now, but need to figure ou tif we need to think about the values in any field other than `payload`
//         let payload = OwnedMessage::new(Some(buf), None, "".into(), Timestamp::now(), 0, 0, None);
//         OwnedMessageNewType(payload)
//     }
// }