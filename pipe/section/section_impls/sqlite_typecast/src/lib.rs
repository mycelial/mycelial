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
                        let mut stream = match stream {
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

                                            let payload = df_to_tc_message(
                                                stream.origin().into(),
                                                df,
                                                stream.ack()
                                            );

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

fn df_to_tc_message(origin: Arc<str>, df: Box<dyn DataFrame>, ack: Ack) -> TypecastMessage {
    let columns: Vec<TableColumn> = df
        .columns()
        .iter()
        .map(|c| TableColumn {
            name: c.name().into(),
            data_type: c.data_type(),
        })
        .collect::<Vec<_>>();

    let values: Vec<Vec<Value>> = df
        .columns()
        .into_iter()
        .map(|col| {
            col.map(|val| {
                let val = (&val).into();
                val
            })
            .collect::<Vec<Value>>()
        })
        .collect();

    let payload = TypecastPayload {
        columns: columns.into(),
        values: values,
    };
    TypecastMessage::new(origin, payload, Some(ack))
}
