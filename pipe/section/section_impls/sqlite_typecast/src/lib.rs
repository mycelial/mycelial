use std::pin::pin;
use std::sync::Arc;

use arrow::array::StringArray;
use arrow::datatypes::DataType as ArrowDataType;
use arrow::datatypes::Field;
use arrow::datatypes::SchemaBuilder;
use arrow::record_batch::RecordBatch as ArrowRecordBatch;

use arrow_msg::df_to_recordbatch;
use arrow_msg::ArrowMsg;

use section::command_channel::{Command, SectionChannel};
use section::futures::{self, Sink, SinkExt, Stream};
use section::futures::{FutureExt, StreamExt};
use section::pretty_print::pretty_print;
use section::section::Section;
use section::{message::Chunk, SectionError, SectionFuture, SectionMessage};

#[derive(Debug)]
pub enum Type {
    Int,
    Float,
    String,
}

#[derive(Debug)]
pub struct SqliteTypecast {
    target_type: Type,
    column: String,
}

impl Default for SqliteTypecast {
    fn default() -> Self {
        Self::new(Type::String, "text")
    }
}

impl SqliteTypecast {
    pub fn new(target_type: Type, column: &str) -> Self {
        Self {
            column: column.to_string(),
            target_type: target_type,
        }
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for SqliteTypecast
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
                        if let Command::Stop = cmd? {
                            return Ok(())
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
                                            section_channel.log(format!("transform got dataframe chunk from {}:\n{}",
                                                stream.origin(),
                                                pretty_print(&*df))).await?;

                                            // // Convert to a RecordBatch from the arrow crate
                                            // let rb = df_to_recordbatch(df.as_ref())?;
                                            // let old_schema = rb.schema();
                                            // let old_cols = rb.columns();

                                            // // Create a new schema from the old schema and push on a new field
                                            // let mut builder = SchemaBuilder::from(old_schema.fields());
                                            // builder.push(Field::new(self.column.clone(), ArrowDataType::Utf8, false));
                                            // let new_schema = builder.finish();

                                            // // Push a new value onto the old columns
                                            // let mut values = old_cols
                                            //     .iter()
                                            //     .map(std::borrow::ToOwned::to_owned)
                                            //     .collect::<Vec<_>>();
                                            // let tag = Arc::new(StringArray::from(vec![self.text.clone()]));
                                            // values.push(tag);

                                            // // create a new arrow RecordBatch from the schema and columns
                                            // let new_rb = ArrowRecordBatch::try_new(
                                            //     Arc::new(new_schema),
                                            //     values,
                                            // )?;

                                            // // create a message from the RecordBatch
                                            // let new_msg = ArrowMsg::new("tagging transformer", vec![Some(new_rb.into())], None);

                                            // output.send(Box::new(df)).await.ok();
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
