//! Origin transformer section
//! Replaces all regex groups with given replacement

use regex::Regex;
use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt, Sink, SinkExt, Stream, StreamExt},
    message::Message,
    section::Section,
    SectionError, SectionFuture, SectionMessage,
};
use std::pin::pin;

#[derive(Debug)]
pub struct OriginTransform {
    regex: Regex,
    replacement: String,
}

impl OriginTransform {
    pub fn new(regex: &str, replacement: &str) -> Result<Self, SectionError> {
        Ok(Self {
            regex: Regex::new(regex)?,
            replacement: replacement.into(),
        })
    }
}

#[derive(Debug)]
struct OriginTransformMsg {
    origin: String,
    inner: SectionMessage,
}

impl OriginTransformMsg {
    fn new(origin: String, inner: SectionMessage) -> Self {
        Self { origin, inner }
    }
}

impl Message for OriginTransformMsg {
    fn ack(&mut self) -> section::message::Ack {
        self.inner.ack()
    }

    fn next(&mut self) -> section::message::Next<'_> {
        self.inner.next()
    }

    fn origin(&self) -> &str {
        self.origin.as_str()
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for OriginTransform
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel,
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
                    },
                    msg = input.next().fuse() => {
                        let msg = match msg {
                            Some(msg) => msg,
                            None => {
                                tracing::info!("input closed");
                                return Ok(())
                            }
                        };
                        let origin = self.regex.replace_all(msg.origin(), &self.replacement);
                        output
                            .send(Box::new(OriginTransformMsg::new(origin.to_string(), msg)))
                            .await
                            .map_err(|_| "output closed")?
                    }
                }
            }
        })
    }
}
