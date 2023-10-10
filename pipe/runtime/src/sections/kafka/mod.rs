pub mod destination;

use crate::message::RecordBatch;
use rdkafka::message::{OwnedMessage, Timestamp};

#[derive(Debug)]
#[repr(transparent)]
pub struct OwnedMessageNewType(OwnedMessage);

impl From<&RecordBatch> for OwnedMessageNewType {
    fn from(arrow_rb: &RecordBatch) -> Self {
        println!("arrow_rb: {:?}", arrow_rb);
        let buf = Vec::new();
        let mut writer = arrow::json::LineDelimitedWriter::new(buf);
        // todo:  this couldn't handle a "binary" column type
        // `JsonError("data type Binary not supported in nested map for json writer")`
        writer.write(&arrow_rb).unwrap();
        writer.finish().unwrap();
        let buf = writer.into_inner();

        // TODO: Basic message works for now, but need to figure ou tif we need to think about the values in any field other than `payload`
        let payload = OwnedMessage::new(
            Some(buf),
            None,
            "".into(),
            Timestamp::now(),
            0,
            0,
            None,
        );
        OwnedMessageNewType(payload)
    }
}
