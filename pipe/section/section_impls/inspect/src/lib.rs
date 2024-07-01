//! Inspect section
use std::pin::pin;

use section::{prelude::*, pretty_print::pretty_print_with_limit};

#[derive(Debug, Default)]
pub struct Inspect {}

#[derive(Debug)]
struct InspectMessage(SectionMessage);

impl Message for InspectMessage {
    fn origin(&self) -> &str {
        self.0.origin()
    }

    fn next(&mut self) -> Next<'_> {
        Box::pin(async {
            let chunk = self.0.next().await;
            match &chunk {
                Ok(None) => tracing::info!("end of stream"),
                Ok(Some(Chunk::DataFrame(df))) => {
                    tracing::info!("\n{}", pretty_print_with_limit(&**df, 64))
                }
                Ok(Some(Chunk::Byte(bin))) => {
                    let len = bin.len();
                    tracing::info!("bin chunk with len: {}", len);
                    if len < 1024 {
                        let string = String::from_utf8_lossy(bin.as_slice());
                        tracing::info!("value:\n{}", string);
                    }
                }
                Err(_) => (),
            };
            chunk
        })
    }

    fn ack(&mut self) -> Ack {
        self.0.ack()
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Inspect
where
    Input: SectionStream,
    Output: SectionSink,
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
                            None => Err("input closed")?,
                            Some(msg) => msg
                        };
                        tracing::info!("new message with origin: {}", msg.origin());
                        output.send(Box::new(InspectMessage(msg))).await.map_err(|_| "send error")?
                    }
                }
            }
        })
    }
}
