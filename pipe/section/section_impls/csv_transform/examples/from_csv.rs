//! Convert binary stream into dataframe

use std::time::Duration;

use csv_transform::source::FromCsv;
use section::{
    dummy::DummySectionChannel,
    futures::SinkExt,
    message::{Chunk, Message},
    pretty_print::pretty_print,
    section::Section, SectionMessage,
};

use tokio::sync::mpsc::channel;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;

#[derive(Debug)]
struct Msg {
    batch_size: usize,
    pos: usize,
    payload: &'static [u8],
}

impl Msg {
    fn new(batch_size: usize) -> Self {
        Self {
            batch_size,
            pos: 0,
            payload: "city,region,country,population
Southborough,MA,United States,9686
Northbridge,MA,United States,14061
Marlborough,MA,United States,38334
Boston,MA,United States,152227
Springfield,MO,United States,150443
Trenton,NJ,United States,14976
Plymouth,NH,United States,42605"
                .as_bytes(),
        }
    }
}

impl Message for Msg {
    fn origin(&self) -> &str {
        "example"
    }

    fn ack(&mut self) -> section::message::Ack {
        Box::pin(async {})
    }

    fn next(&mut self) -> section::message::Next<'_> {
        Box::pin(async move {
            if self.pos >= self.payload.len() {
                return Ok(None);
            }
            let (chunk, pos) = if self.pos + self.batch_size > self.payload.len() {
                (&self.payload[self.pos..], self.payload.len())
            } else {
                let end = self.pos + self.batch_size;
                (&self.payload[self.pos..end], end)
            };
            self.pos = pos;
            Ok(Some(Chunk::Byte(chunk.into())))
        })
    }
}

#[tokio::main]
async fn main() {
    let source = FromCsv::new(2);

    let (tx_in, rx_in) = channel::<SectionMessage>(1);
    let rx_in = ReceiverStream::new(rx_in);
    let (tx_out, mut rx_out) = channel::<SectionMessage>(1);
    let tx_out = PollSender::new(tx_out).sink_map_err(|_| "send error".into());
    let mut interval = tokio::time::interval(Duration::from_secs(5));

    tokio::spawn(async move {
        source
            .start(
                Box::new(rx_in),
                Box::new(tx_out),
                DummySectionChannel::new(),
            )
            .await
            .unwrap();
    });

    loop {
        tokio::select! {
            _ = interval.tick() => {
                tx_in.send(Box::new(Msg::new(1))).await.unwrap();
            },
            msg = rx_out.recv() => {
                let mut msg = msg.unwrap();
                while let Some(chunk) = msg.next().await.unwrap() {
                    match chunk {
                        Chunk::DataFrame(df) => println!("{}", pretty_print(&*df)),
                        Chunk::Byte(_bin) => println!("bin"),
                    };
                }
                msg.ack().await;
            }
        }
    }
}
