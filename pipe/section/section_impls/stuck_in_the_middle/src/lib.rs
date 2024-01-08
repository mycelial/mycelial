use std::pin::pin;
use std::sync::Arc;

use arrow::array::ArrayData;
use arrow::array::ArrayRef;
use arrow::array::StringArray;
use arrow::datatypes::DataType as ArrowDataType;
use arrow::datatypes::Field;
use arrow::datatypes::Schema;
use arrow::datatypes::SchemaBuilder;
use arrow::record_batch::RecordBatch as ArrowRecordBatch;

use arrow_msg::ArrowMsg;
use arrow_msg::{df_to_recordbatch, RecordBatch};

use section::command_channel::{Command, SectionChannel};
use section::futures::{self, Sink, SinkExt, Stream};
use section::futures::{FutureExt, StreamExt};
use section::pretty_print::pretty_print;
use section::section::Section;
use section::{
    message::{Chunk, Column, DataFrame, DataType, Message, ValueView},
    SectionError, SectionFuture, SectionMessage,
};

#[derive(Debug)]
pub struct StuckInTheMiddle {
    count: usize,
}
impl Default for StuckInTheMiddle {
    fn default() -> Self {
        Self::new()
    }
}

impl StuckInTheMiddle {
    pub fn new() -> Self {
        Self { count: 0 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SITMPayload<'a> {
    /// message
    pub message: String,
    pub other_cols: Vec<Col<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
struct Col<'a> {
    pub name: String,
    pub data_type: DataType,
    pub data: Vec<ValueView<'a>>,
}

impl DataFrame for SITMPayload<'static> {
    fn columns(&self) -> Vec<Column<'_>> {
        vec![Column::new(
            "message",
            DataType::Str,
            Box::new(std::iter::once(ValueView::from(&self.message))),
        )]
    }
}

impl<'a> SITMPayload<'a> {
    fn from(_df: impl DataFrame) {}
}

#[derive(Debug)]
struct Once {
    inner: Option<Box<dyn DataFrame>>,
}

impl Message for Once {
    fn origin(&self) -> &str {
        "hello world"
    }

    fn next(&mut self) -> section::message::Next<'_> {
        let v = self.inner.take().map(Chunk::DataFrame);
        Box::pin(async move { Ok(v) })
    }

    fn ack(&mut self) -> section::message::Ack {
        Box::pin(async {})
    }
}

// impl From<SITMPayload<'_>> for SectionMessage {
//     fn from(val: SITMPayload) -> Self {
//         Box::new(Once {
//             inner: Some(Box::new(val)),
//         })
//     }
// }

fn to_sm(val: SITMPayload<'static>) -> SectionMessage {
    Box::new(Once {
        inner: Some(Box::new(val)),
    })
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for StuckInTheMiddle
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel + Send + Sync + 'static,
{
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(self, input: Input, output: Output, mut section_channel: SectionChan) -> Self::Future {
        println!("stuck_in_the_middle event loop started!");

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

                                            // Convert to a RecordBatch from the arrow crate
                                            let rb = df_to_recordbatch(df)?;
                                            let old_schema = rb.schema();
                                            let old_cols = rb.columns();

                                            // Create a new schema from the old schema and push on a new field
                                            let mut builder = SchemaBuilder::from(old_schema.fields());
                                            builder.push(Field::new("name", ArrowDataType::Utf8, false));
                                            let new_schema = builder.finish();

                                            // Push a new value onto the old columns
                                            let mut values = old_cols
                                                .into_iter()
                                                .map(|c| c.to_owned())
                                                .collect::<Vec<_>>();
                                            let tag = Arc::new(StringArray::from(vec!["sitm tag"]));
                                            values.push(tag);

                                            // create a new arrow RecordBatch from the schema and columns
                                            let new_rb = ArrowRecordBatch::try_new(
                                                Arc::new(new_schema),
                                                values,
                                            )?;

                                            // create a message from the RecordBatch
                                            let new_msg = ArrowMsg::new("sitm", vec![Some(new_rb.into())], None);

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

// impl Stream for StuckInTheMiddle {
//     type Item = usize;

//     fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//         Poll::Pending
//     }
// }

// impl<T> Sink<T> for StuckInTheMiddle {

//     fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         Poll::Ready(Ok(()))
//     }

//     fn start_send(self: Pin<&mut Self>, _item: T) -> Result<(), Self::Error> {
//         Ok(())
//     }

//     fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         Poll::Ready(Ok(()))
//     }

//     fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         Poll::Ready(Ok(()))
//     }
// }
