//! Bacalhau Source Section
//!
//! Monitors the configured folder on disk for file events, and then
//! after reading the entire JSON blob into a messages, sends it on
//! to a receiving destination section.

use crate::{
    config::Map,
    types::{DynSection, DynSink, DynStream, SectionError, SectionFuture},
};

use arrow::record_batch::RecordBatch;
use bacalhau::source::Bacalhau;
use futures::SinkExt;
use section::{Section, SectionChannel};
use stub::Stub;

#[derive(Debug)]
pub struct BacalhauAdapter {
    inner: Bacalhau,
}

impl<SectionChan: SectionChannel + Send + 'static> Section<DynStream, DynSink, SectionChan>
    for BacalhauAdapter
{
    // FIXME: define proper error
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(
        self,
        _input: DynStream,
        output: DynSink,
        section_channel: SectionChan,
    ) -> Self::Future {
        Box::pin(async move {
            let input = Stub::<bacalhau::Message, SectionError>::new();
            let output = output.with(|message: bacalhau::Message| async {
                let payload: RecordBatch = message.payload.try_into()?;
                let message = section::Message::new(message.origin, payload, message.ack);
                Ok(message)
            });
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
