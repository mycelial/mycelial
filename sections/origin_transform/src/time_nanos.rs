//! Origin transformer section
//! Replaces all regex groups with timestamp in nanoseconds

use regex::Regex;
use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt, Sink, SinkExt, Stream, StreamExt},
    section::Section,
    SectionError, SectionFuture, SectionMessage,
};
use std::{
    pin::pin,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::OriginTransformMsg;

#[derive(Debug)]
pub struct OriginTransform {
    regex: Regex,
}

impl OriginTransform {
    pub fn new(regex: &str) -> Result<Self, SectionError> {
        Ok(Self {
            regex: Regex::new(regex)?,
        })
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
                        // FIXME: system time is not monotonic
                        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                        let replacement = format!("{}", now.as_nanos());
                        let origin = self.regex.replace_all(msg.origin(), &replacement);
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
