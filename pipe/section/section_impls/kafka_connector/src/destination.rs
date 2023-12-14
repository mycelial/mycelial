// Kafka destination section implementation
// CAUTION: ALPHA QUALITY CODE :) Use with caution.

use crate::{Message, StdError};
use rdkafka::message::OwnedMessage;
use rdkafka::producer::FutureRecord;
use rdkafka::util::Timeout;
use rdkafka::Message as KafkaMessage;
use rdkafka::{error::KafkaError, producer::FutureProducer, ClientConfig};
use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt, Sink, SinkExt, Stream, StreamExt},
    message::Value,
    section::Section,
    state::State,
    SectionError, SectionMessage,
};
use std::pin::{pin, Pin};

use std::future::Future;

pub struct Kafka {
    producer: FutureProducer,
    topic: String,
}

impl Kafka {
    pub fn new(brokers: &str, topic: &str) -> Self {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .create().unwrap();
        Self {
            producer,
            topic: topic.into(),
        }
    }

    async fn enter_loop<Input, Output, SectionChan>(
        self,
        input: Input,
        output: Output,
        mut section_channel: SectionChan,
    ) -> Result<(), SectionError>
    where
        Input: Stream + Send,
        Output: Sink<SectionMessage, Error = SectionError> + Send,
        SectionChan: SectionChannel + Send + Sync,
    {
        let mut input = pin!(input.fuse());

        loop {
            futures::select! {
                cmd = section_channel.recv().fuse() => {
                    if let Command::Stop = cmd? { return Ok(()) }
                },
                message = input.next() => {
                    match message {
                        Some(mut msg) => {
                            // let payload: &OwnedMessage = &msg.payload;
                            // let origin = &msg.origin;

                            //  let p = payload.payload();
                            //  let p = match p {
                            //         Some(p) => p,
                            //         None => Err("payload is none")?,
                            //  };

                            // let record = FutureRecord::to(self.topic.as_str())
                            //     .payload(p)
                            //     .key(origin.as_bytes());

                            // if self.producer.send(record, Timeout::Never).await.is_err() {
                            //     return Ok(())
                            // }

                            // msg.ack().await;
                        },
                        None => Err("input closed")?,
                    }
                },
            }
        }
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Kafka
where
    Input: Stream + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel + Send + Sync + 'static,
{
    type Error = SectionError;
    type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'static>>;

    fn start(self, input: Input, output: Output, command: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, command).await })
    }
}
