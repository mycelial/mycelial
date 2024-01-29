use std::pin::pin;
use std::sync::Arc;

use section::command_channel::{Command, SectionChannel};
use section::futures::{self, Sink, SinkExt, Stream};
use section::futures::{FutureExt, StreamExt};
use section::pretty_print::pretty_print;
use section::section::Section;
use section::{
    message::{Ack, Chunk, Column, DataFrame, DataType, Message, Value},
    SectionError, SectionFuture, SectionMessage,
};

#[derive(Debug)]
pub enum Type {
    Int,
    Real,
    String,
}

#[derive(Debug)]
pub struct TypecastTransformer {
    target_type: DataType,
    column: String,
}

impl Default for TypecastTransformer {
    fn default() -> Self {
        Self::new(DataType::Str, "text")
    }
}

impl TypecastTransformer {
    pub fn new(target_type: DataType, column: &str) -> Self {
        Self {
            column: column.to_string(),
            target_type,
        }
    }

    fn df_to_tc_message(&self, origin: Arc<str>, df: &dyn DataFrame, ack: Ack) -> TypecastMessage {
        let mut casted_column: Option<usize> = None;

        let columns: Vec<TableColumn> = df
            .columns()
            .iter()
            .enumerate()
            .map(|(i, column)| {
                let name: Arc<str> = column.name().into();

                let data_type = if self.column.eq(&*name) {
                    casted_column = Some(i);
                    // todo: check that the column can actually be casted to
                    //column.data_type()
                    self.target_type
                } else {
                    column.data_type()
                };
                TableColumn { name, data_type }
            })
            .collect::<Vec<_>>();

        let values: Vec<Vec<Value>> = df
            .columns()
            .into_iter()
            .enumerate()
            .map(|(i, col)| {
                col.map(|val| {
                    if casted_column == Some(i) {
                        let value: Value = (&val).into();
                        // we do the cast
                        let casted = match self.target_type {
                            DataType::I64 => value.into_i64(),
                            DataType::F64 => value.into_f64(),
                            // DataType::Str => value.into_string(),
                            _ => todo!(),
                        };
                        casted.unwrap_or_else(|_| {
                            let msg =
                                format!("could not convert {:?} to {:?}", val, self.target_type);
                            Value::Str(msg.to_owned().into_boxed_str())
                        })
                    } else {
                        // we pass along the old value
                        (&val).into()
                    }
                })
                .collect::<Vec<Value>>()
            })
            .collect();

        let payload = TypecastPayload {
            columns: columns.into(),
            values,
        };
        TypecastMessage::new(origin, payload, Some(ack))
    }
}

// datatype
#[derive(Debug)]
#[allow(unused)]
pub(crate) struct Table {
    name: Arc<str>,
    columns: Arc<[TableColumn]>,
    query: String,
    offset: i64,
    limit: i64,
}

#[derive(Debug)]
#[allow(unused)]
pub(crate) struct TableColumn {
    name: Arc<str>,
    data_type: DataType,
}

#[derive(Debug)]
pub(crate) struct TypecastPayload {
    columns: Arc<[TableColumn]>,
    values: Vec<Vec<Value>>,
}

impl DataFrame for TypecastPayload {
    fn columns(&self) -> Vec<section::message::Column<'_>> {
        self.columns
            .iter()
            .zip(self.values.iter())
            .map(|(col, column)| {
                Column::new(
                    col.name.as_ref(),
                    col.data_type,
                    Box::new(column.iter().map(Into::into)),
                )
            })
            .collect()
    }
}

pub struct TypecastMessage {
    origin: Arc<str>,
    payload: Option<Box<dyn DataFrame>>,
    ack: Option<Ack>,
}

impl TypecastMessage {
    fn new(origin: Arc<str>, payload: impl DataFrame, ack: Option<Ack>) -> Self {
        Self {
            origin,
            payload: Some(Box::new(payload)),
            ack,
        }
    }
}

impl std::fmt::Debug for TypecastMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("typecastMessage")
            .field("origin", &self.origin)
            .field("payload", &self.payload)
            .finish()
    }
}

impl Message for TypecastMessage {
    fn origin(&self) -> &str {
        self.origin.as_ref()
    }

    fn next(&mut self) -> section::message::Next<'_> {
        let v = self.payload.take().map(Chunk::DataFrame);
        Box::pin(async move { Ok(v) })
    }

    fn ack(&mut self) -> Ack {
        self.ack.take().unwrap_or(Box::pin(async {}))
    }
}

//

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for TypecastTransformer
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
                        let mut stream = match stream {
                            Some(stream) => stream,
                            None => Err("input stream closed")?
                        };
                        loop {
                            futures::select! {
                                msg = stream.next().fuse() => {
                                    match msg? {
                                        Some(Chunk::DataFrame(df)) => {
                                            let payload = self.df_to_tc_message(
                                                stream.origin().into(),
                                                &*df,
                                                stream.ack()
                                            );

                                            section_channel.log(format!("transform got dataframe chunk from {}:\n{}",
                                                stream.origin(),
                                                pretty_print(&*df))).await?;

                                            output.send(Box::new(payload)).await.ok();
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
