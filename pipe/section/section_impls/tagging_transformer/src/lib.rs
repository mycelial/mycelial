use std::pin::pin;
use std::sync::Arc;

use anyhow::{anyhow, Result};

use arrow::array::StringArray;
use arrow::datatypes::DataType as ArrowDataType;
use arrow::datatypes::Field;
use arrow::datatypes::SchemaBuilder;
use arrow::record_batch::RecordBatch as ArrowRecordBatch;

use arrow_msg::df_to_recordbatch;
use arrow_msg::ArrowMsg;

use section::command_channel::{Command, SectionChannel, WeakSectionChannel};
use section::futures::{self, Sink, SinkExt, Stream};
use section::futures::{FutureExt, StreamExt};
use section::message::Ack;
use section::message::DataFrame;
use section::section::Section;
use section::{message::Chunk, SectionError, SectionFuture, SectionMessage};

#[derive(Debug)]
pub struct TaggingTransformer {
    column: String,
    text: String,
}

impl Default for TaggingTransformer {
    fn default() -> Self {
        Self::new("tag", "text")
    }
}

impl TaggingTransformer {
    pub fn new(column: &str, text: &str) -> Self {
        Self {
            column: column.to_string(),
            text: text.to_string(),
        }
    }

    /// Handles an incoming DataFrame message by adding a column with the configured text
    async fn handle_message(&self, df: Box<dyn DataFrame>, ack: Ack) -> Result<ArrowMsg> {
        // Convert to a RecordBatch from the arrow crate
        let rb = match df_to_recordbatch(df) {
            Ok(rb) => rb,
            Err(_) => return Err(anyhow!("couldn't parse DataFrame into RecordBatch")),
        };
        let old_schema = rb.schema();
        let old_cols = rb.columns();

        // Create a new schema from the old schema and push on a new field
        let mut builder = SchemaBuilder::from(old_schema.fields());
        builder.push(Field::new(self.column.clone(), ArrowDataType::Utf8, false));
        let new_schema = builder.finish();

        // Push a new value onto the old columns
        let mut values = old_cols
            .iter()
            .map(std::borrow::ToOwned::to_owned)
            .collect::<Vec<_>>();
        let tag = Arc::new(StringArray::from(vec![self.text.clone()]));
        values.push(tag);

        // create a new arrow RecordBatch from the schema and columns
        let new_rb = ArrowRecordBatch::try_new(Arc::new(new_schema), values)?;

        // create a message from the RecordBatch
        Ok(ArrowMsg::new(
            "tagging transformer",
            vec![Some(new_rb.into())],
            Some(ack),
        ))
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for TaggingTransformer
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel + Send + Sync + 'static,
{
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(self, input: Input, output: Output, mut section_channel: SectionChan) -> Self::Future {
        Box::pin(async move {
            let mut input = pin!(input.fuse());
            let mut output = pin!(output);

            loop {
                futures::select! {
                    cmd = section_channel.recv().fuse() => {
                        match cmd? {
                            Command::Ack(ack_msg) => {
                                let weak_chan = section_channel.weak_chan();
                                weak_chan.ack(Box::new(ack_msg)).await;

                            },
                            Command::Stop => return Ok(()),
                            _ => {},
                        }
                    }
                    stream = input.next() => {
                        let mut stream = match stream{
                            Some(stream) => stream,
                            None => Err("input stream closed")?
                        };
                        loop {
                            futures::select! {
                                msg = stream.next().fuse() => {
                                    match msg? {
                                        Some(Chunk::DataFrame(df)) => {
                                            let new_msg = self.handle_message(df, stream.ack()).await?;
                                            output.send(Box::new(new_msg)).await.ok();
                                            section_channel.log("payload sent").await?;
                                        },
                                        Some(_) => {Err("unsupported stream type, dataframe expected")?},
                                        None => break,
                                    }
                                },
                                cmd = section_channel.recv().fuse() => {
                                    if let Command::Stop = cmd? {
                                        return Ok(())
                                    }
                                }
                            }
                        }
                    }
                }
            }
        })
    }
}
