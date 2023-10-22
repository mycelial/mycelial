//! Bacalhau Destination Ssection
//!
//! Receives messages from source sections, and after extracting
//! information from the RecordBatch, posts it to the configured
//! endpoint, to be processed by an external process.
use crate::config::Map;
use crate::types::{DynSection, DynSink, DynStream, SectionError};

use futures::StreamExt;
use std::future::Future;
use std::pin::Pin;
use stub::Stub;

use bacalhau::{destination::Bacalhau, BacalhauPayload};
use section::{Section, SectionChannel};

#[derive(Debug)]
pub struct BacalhauAdapter {
    inner: Bacalhau,
}

impl<SectionChan: SectionChannel + Send + 'static> Section<DynStream, DynSink, SectionChan>
    for BacalhauAdapter
{
    // FIXME: define proper error
    type Error = SectionError;
    type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send>>;

    fn start(
        self,
        input: DynStream,
        _output: DynSink,
        section_channel: SectionChan,
    ) -> Self::Future {
        Box::pin(async move {
            let input = input.map(|message: section::Message<crate::message::RecordBatch>| {
                let bac: BacalhauPayload = (&message.payload.0).try_into().unwrap();
                bacalhau::Message::new(message.origin, bac, message.ack)
            });
            let output = Stub::<bacalhau::Message, SectionError>::new();
            self.inner.start(input, output, section_channel).await
        })
    }
}

pub fn constructor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let job = config
        .get("job")
        .ok_or("bacalhau section requires 'job'")?
        .as_str()
        .ok_or("'job' should be a string")?;
    let endpoint = config
        .get("endpoint")
        .ok_or("bacalhau section requires 'endpoint'")?
        .as_str()
        .ok_or("'endpoint' should be a string")?;
    let outputs = config
        .get("outputs")
        .ok_or("bacalhau section requires 'outputs'")?
        .as_str()
        .ok_or("'outputs' should be a string")?;
    Ok(Box::new(BacalhauAdapter {
        inner: Bacalhau::new(job, endpoint, outputs),
    }))
}
