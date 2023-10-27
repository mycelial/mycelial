use section::SectionChannel;
use crate::message::{Message, RecordBatch};
use futures::SinkExt;
use section::Section;
use excel_connector::source::Excel;

use crate::types::SectionFuture;
use crate::{
    config::Map,
    types::{DynSection, DynSink, DynStream, SectionError},
};

#[allow(dead_code)]
pub struct ExcelAdapter {
    inner: Excel,
}

impl<SectionChan: SectionChannel + Send + 'static> Section<DynStream, DynSink, SectionChan> for ExcelAdapter {
    type Future = SectionFuture;
    type Error = SectionError;

    fn start(
        self,
        input: DynStream,
        output: DynSink,
        section_channel: SectionChan,
    ) -> Self::Future {
        Box::pin(async move {
            let output = output.with(|message: excel_connector::Message| async {
                let payload: RecordBatch = message.payload.try_into()?;
                let message = Message::new(message.origin, payload, message.ack);
                Ok(message)
            });
            self.inner.start(input, output, section_channel).await
        })
    }
}

/// constructor for Excel
pub fn constructor<S: SectionChannel>(config: &Map) -> Result<Box<dyn DynSection<S>>, SectionError> {
    // FIXME: Use correct input config values for excel
    let sheets = config
        .get("sheets")
        .ok_or("excel section requires 'sheets'")?
        .as_str()
        .ok_or("'sheets' should be string")?;
    let path = config
        .get("path")
        .ok_or("excel section requires 'path'")?
        .as_str()
        .ok_or("path should be string")?;
    let sheets = sheets
        .split(',')
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .collect::<Vec<&str>>();
    Ok(Box::new(ExcelAdapter {
        inner: Excel::new(path, sheets.as_slice()),
    }))
}
