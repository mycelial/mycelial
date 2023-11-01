//! HelloWorld Destination/middleware example section implementation
//!
//! Upon receipt of a message, this section will log "Hello, World!" + the message received. It
//! then forwards the message on to the next section.
use futures::{FutureExt, Sink, SinkExt, Stream, StreamExt};
use section::{Command, Section, SectionChannel};
use std::future::Future;

use std::pin::{pin, Pin};

use crate::{
    config::Map,
    message::Message,
    types::{DynSection, SectionError},
};

impl Default for HelloWorld {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct HelloWorld {}

impl HelloWorld {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn enter_loop<Input, Output, SectionChan>(
        self,
        input: Input,
        output: Output,
        mut section_chan: SectionChan,
    ) -> Result<(), SectionError>
    where
        Input: Stream<Item = Message> + Send + 'static,
        Output: Sink<Message, Error = SectionError> + Send + 'static,
        SectionChan: SectionChannel + Send + Sync + 'static,
    {
        let mut input = pin!(input.fuse());
        let mut output = pin!(output);
        loop {
            futures::select_biased! {
                cmd = section_chan.recv().fuse() => {
                    if let Command::Stop = cmd? {
                        return Ok(())
                    }
                }
                msg = input.next() => {
                    let msg = match msg {
                        Some(msg) => msg,
                        None => Err("input stream closed")?
                    };

                    let payload = &msg.payload;
                    let origin = &msg.origin;
                    section_chan.log(&format!("Message from '{:?}' received! {:?}", origin, payload)).await?;
                    output.send(msg).await?;
                },
            }
        }
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for HelloWorld
where
    Input: Stream<Item = Message> + Send + 'static,
    Output: Sink<Message, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel + Send + Sync + 'static,
{
    // FIXME: define proper error
    type Error = SectionError;
    type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send>>;

    fn start(self, input: Input, output: Output, section_chan: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, section_chan).await })
    }
}

pub fn constructor<S: SectionChannel>(
    _config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    Ok(Box::new(HelloWorld::new()))
}
