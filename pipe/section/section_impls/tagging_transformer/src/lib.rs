use std::pin::pin;
use std::sync::Arc;

use section::command_channel::{Command, SectionChannel};
use section::futures::{self, Sink, SinkExt, Stream};
use section::futures::{FutureExt, StreamExt};
use section::message::{Column, DataFrame, DataType, Message, ValueView};
use section::section::Section;
use section::{message::Chunk, SectionError, SectionFuture, SectionMessage};

#[derive(Debug)]
pub struct TaggingTransformer {
    column: Arc<str>,
    text: Arc<str>,
}

impl TaggingTransformer {
    pub fn new(column: &str, text: &str) -> Self {
        Self {
            column: Arc::from(column),
            text: Arc::from(text),
        }
    }
}

#[derive(Debug)]
struct Msg {
    inner: SectionMessage,
    column: Arc<str>,
    text: Arc<str>,
}

impl Msg {
    fn new(inner: SectionMessage, column: Arc<str>, text: Arc<str>) -> Self {
        Self {
            inner,
            column,
            text,
        }
    }
}

#[derive(Debug)]
struct TaggingDf {
    inner: Box<dyn DataFrame>,
    column: Arc<str>,
    text: Arc<str>,
}

impl DataFrame for TaggingDf {
    fn columns(&self) -> Vec<Column<'_>> {
        // FIXME: would be nice to have size hint
        let len = self
            .inner
            .columns()
            .pop()
            .map(|col| col.count())
            .unwrap_or(0);

        let mut columns = self.inner.columns();
        // checking if column is already present
        match columns.iter().any(|col| col.name() == self.column.as_ref()) {
            true => {
                tracing::error!(
                    "tagging transformer can't add already existing column: {}",
                    self.column
                )
            }
            false => {
                columns.push(Column::new(
                    &self.column,
                    DataType::Str,
                    Box::new(std::iter::repeat(ValueView::Str(&self.text)).take(len)),
                ));
            }
        }
        columns
    }
}

impl Message for Msg {
    fn ack(&mut self) -> section::message::Ack {
        self.inner.ack()
    }

    fn origin(&self) -> &str {
        self.inner.origin()
    }

    fn next(&mut self) -> section::message::Next<'_> {
        Box::pin(async {
            match self.inner.next().await {
                Ok(None) => Ok(None),
                Ok(Some(Chunk::DataFrame(df))) => {
                    let df = TaggingDf {
                        inner: df,
                        column: Arc::clone(&self.column),
                        text: Arc::clone(&self.text),
                    };
                    Ok(Some(Chunk::DataFrame(Box::new(df))))
                }
                // FIXME: error out
                Ok(res @ Some(Chunk::Byte(_))) => Ok(res),
                Err(e) => Err(e),
            }
        })
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
            let mut input = pin!(input);
            let mut output = pin!(output);

            loop {
                futures::select! {
                    cmd = section_channel.recv().fuse() => {
                        if let Command::Stop = cmd? {
                            return Ok(())
                        }
                    }
                    msg = input.next().fuse() => {
                        let msg = match msg{
                            Some(msg) => msg,
                            None => Err("input stream closed")?
                        };
                        let msg = Msg::new(msg, Arc::clone(&self.column), Arc::clone(&self.text));
                        println!("msg: {:?}", msg);
                        output.send(Box::new(msg)).await?;
                    }
                }
            }
        })
    }
}
