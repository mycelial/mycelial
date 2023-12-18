// Kafka destination section implementation
// CAUTION: ALPHA QUALITY CODE :) Use with caution.

use rdkafka::producer::FutureRecord;
use rdkafka::util::Timeout;
use rdkafka::{producer::FutureProducer, ClientConfig};
use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt, Sink, Stream, StreamExt},
    message::{Chunk, ValueView},
    section::Section,
    SectionError, SectionMessage,
};
use std::collections::HashMap;
use std::pin::{pin, Pin};

use serde_json::json;
use std::future::Future;

pub struct Kafka {
    // todo: Add an option to represent how the data should be serialized (e.g. json, avro, etc.)
    producer: FutureProducer,
    topic: String,
}

impl Kafka {
    pub fn new(brokers: &str, topic: &str) -> Self {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .create()
            .unwrap();
        Self {
            producer,
            topic: topic.into(),
        }
    }

    async fn enter_loop<Input, Output, SectionChan>(
        self,
        input: Input,
        _output: Output,
        mut section_channel: SectionChan,
    ) -> Result<(), SectionError>
    where
        Input: Stream<Item = SectionMessage> + Send + 'static,
        Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
        SectionChan: SectionChannel + Send + 'static,
    {
        let mut input = pin!(input.fuse());

        loop {
            futures::select! {
                cmd = section_channel.recv().fuse() => {
                    if let Command::Stop = cmd? { return Ok(()) }
                },
                message = input.next() => {
                    let mut message = match message {
                        Some(message) => message,
                        None => Err("input closed")?,
                    };
                    // let origin = message.origin();
                    loop {
                        futures::select! {
                            chunk = message.next().fuse() => {
                                let df = match chunk? {
                                    None => break,
                                    Some(Chunk::DataFrame(df)) => df,
                                    Some(_ch) => continue,
                                };
                                let columns = &mut df.columns();

                                'hello: loop {
                                    let mut payload: HashMap<String, serde_json::Value> = HashMap::new();
                                    for col in columns.iter_mut() {
                                        let val = col.next();
                                        let val = match val {
                                            Some(v) => v,
                                            None => break 'hello,
                                        };
                                        let v = match val {
                                            ValueView::Str(v) => serde_json::Value::String(v.to_string()),
                                            ValueView::I8(i) => serde_json::Value::Number(serde_json::Number::from(i)),
                                            ValueView::I16(i) => serde_json::Value::Number(serde_json::Number::from(i)),
                                            ValueView::I32(i) => serde_json::Value::Number(serde_json::Number::from(i)),
                                            ValueView::I64(i) => serde_json::Value::Number(serde_json::Number::from(i)),
                                            ValueView::U8(i) => serde_json::Value::Number(serde_json::Number::from(i)),
                                            ValueView::U16(i) => serde_json::Value::Number(serde_json::Number::from(i)),
                                            ValueView::U32(i) => serde_json::Value::Number(serde_json::Number::from(i)),
                                            ValueView::U64(i) => serde_json::Value::Number(serde_json::Number::from(i)),
                                            ValueView::F32(f) => serde_json::Value::Number(serde_json::Number::from_f64(f as f64).unwrap()),
                                            ValueView::F64(f) => serde_json::Value::Number(serde_json::Number::from_f64(f).unwrap()),
                                            ValueView::Bin(b) => serde_json::Value::String(std::str::from_utf8(b).unwrap().to_string()),
                                            ValueView::Bool(b) => serde_json::Value::Bool(b),
                                            ValueView::Null => serde_json::Value::Null,
                                            unimplemented => unimplemented!("unimplemented value: {:?}", unimplemented),
                                        };
                                        payload.insert(col.name().to_string(), v);
                                    }

                                    let payload_json = json!(payload).to_string();
                                    let payload_bytes = payload_json.as_bytes();

                                    let record = FutureRecord::to(self.topic.as_str())
                                        .payload(payload_bytes)
                                        .key("origin");

                                    match self.producer.send(record, Timeout::Never).await {
                                        Ok(_) => continue,
                                        Err(e) => println!("error: {:?}", e),

                                    }
                                }
                            },
                        }
                    }
                    message.ack().await;

                },
            }
        }
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Kafka
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel + Send + 'static,
{
    type Error = SectionError;
    type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'static>>;

    fn start(self, input: Input, output: Output, command: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, command).await })
    }
}
