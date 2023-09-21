//! Mycelial Net
//! 
//! network section, dumps incoming messages to provided http endpoint
use arrow::ipc::writer::StreamWriter;
use bytes::Bytes;
use futures::{Sink, Stream, StreamExt};
use section::{State, SectionChannel, Section};
use std::future::Future;

use base64::engine::{general_purpose::STANDARD as BASE64, Engine};
use std::pin::{pin, Pin};
use std::time::Duration;

use crate::{
    message::Message,
    types::{DynSection, SectionError},
    config::Map
};

#[derive(Debug)]
pub struct Mycelial {
    endpoint: String,
    token: String,
    topic: String,
}

impl Mycelial {
    pub fn new(endpoint: impl Into<String>, token: impl Into<String>, topic: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            token: token.into(),
            topic: topic.into(),
        }
    }

    pub async fn enter_loop<Input, Output, SectionChan>(
        self, 
        input: Input,
        _output: Output,
        _section_chan: SectionChan
    ) -> Result<(), SectionError>
    where Input: Stream<Item=Message> + Send + 'static,
          Output: Sink<Message, Error=SectionError> + Send + 'static,
          SectionChan: SectionChannel + Send + Sync + 'static,
    {
        let mut input = pin!(input.fuse());
        let client = &mut reqwest::Client::new();
        while let Some(mut msg) = input.next().await {
            // FIXME: error / unwrap
            let mut stream_writer: StreamWriter<_> =
                StreamWriter::try_new(vec![], msg.payload.0.schema().as_ref()).unwrap();

            // FIXME: unwrap
            stream_writer.write(&msg.payload).unwrap();
            stream_writer.finish().unwrap();

            let bytes: Bytes = stream_writer.into_inner().unwrap().into();
            loop {
                match client
                    .post(format!("{}/{}", self.endpoint.as_str().trim_end_matches('/'), self.topic))
                    .header("Authorization", self.basic_auth())
                    .header("x-message-origin", &msg.origin)
                    .body(bytes.clone())
                    .send()
                    .await
                {
                    Err(e) => {
                        println!("error: {:?}", e);
                        tokio::time::sleep(Duration::from_secs(3)).await;
                    },
                    Ok(res) if res.status() == 200 => {
                        break
                    },
                    Ok(res) => Err(format!("unexpected status code: {}", res.status()))?
                }
            }
            msg.ack().await;
        }
        Ok(())
    }

    fn basic_auth(&self) -> String {
        format!("Basic {}", BASE64.encode(format!("{}:", self.token)))
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Mycelial
    where Input: Stream<Item=Message> + Send + 'static,
          Output: Sink<Message, Error=SectionError> + Send + 'static,
          SectionChan: SectionChannel + Send + Sync + 'static,
{
    // FIXME: define proper error
    type Error = SectionError;
    type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send>>;

    fn start(self, input: Input, output: Output, section_chan: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, section_chan).await })
    }
}

/// constructor for mycelial net section
///
/// # Config example:
/// ```toml
/// [[section]]
/// name = "mycelial_net"
/// endpoint = "http://localhost:8080/ingestion"
/// token = "token"
/// ```
pub fn constructor<S: State>(config: &Map) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let endpoint = config
        .get("endpoint")
        .ok_or("mycelial net section requires 'endpoint' url")?
        .as_str()
        .ok_or("endpoint should be string")?;
    let token = config
        .get("token")
        .ok_or("mycelial net section requires 'token'")?
        .as_str()
        .ok_or("token should be string")?;
    let topic = config
        .get("topic")
        .ok_or("mycelial net section requires 'topic'")?
        .as_str()
        .ok_or("topic should be string")?;
    Ok(Box::new(Mycelial::new(endpoint, token, topic)))
}
