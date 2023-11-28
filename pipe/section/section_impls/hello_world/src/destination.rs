//! HelloWorld Destination/middleware example section implementation
//!
//! Upon receipt of a message, this section will log "Hello, World!" + the message received. It
//! then forwards the message on to the next section.
use std::pin::pin;

use section::futures::StreamExt;
use section::message::Chunk;
use section::pretty_print::pretty_print;
use section::{
    SectionMessage,
    SectionError,
    SectionFuture,
    command_channel::{Command, SectionChannel},
    section::Section,
    futures::{Stream, Sink, FutureExt, self},
};

#[derive(Debug)]
pub struct HelloWorld {}

impl HelloWorld {
    pub fn new() -> Self {
        Self {}
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for HelloWorld
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel + Send + Sync + 'static,
{
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(self, input: Input, _output: Output, mut section_chan: SectionChan) -> Self::Future {
        Box::pin(async move {
            let mut input = pin!(input.fuse());
            loop {
                futures::select! {
                    cmd = section_chan.recv().fuse() => {
                        if let Command::Stop = cmd? {
                            return Ok(())
                        }
                    }
                    stream = input.next() => {
                        let mut stream = match stream{
                            Some(stream) => stream,
                            None => Err("input stream closed")?
                        };
                        'inner: loop {
                            futures::select! {
                                msg = stream.next().fuse() => {
                                    match msg? {
                                        Some(Chunk::DataFrame(df)) => {
                                            section_chan.log(format!("got dataframe chunk from {}:\n{}", stream.origin(), pretty_print(&*df))).await?;
                                        },
                                        Some(_) => {Err("unsupported stream type, dataframe expected")?},
                                        None => break 'inner,
                                    }
                                },
                                cmd = section_chan.recv().fuse() => {
                                    if let Command::Stop = cmd? {
                                        return Ok(())
                                    }
                                }
                            }
                        }
                    },
                }
            }
        })
    }
}
