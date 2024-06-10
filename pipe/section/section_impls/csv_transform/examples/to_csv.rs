//! Convert binary stream into dataframe

use std::time::Duration;

use csv_transform::destination::ToCsv;
use section::{
    dummy::DummySectionChannel,
    futures::SinkExt,
    message::{Chunk, Column, DataFrame, DataType, Message, ValueView},
    section::Section,
    SectionMessage,
};

use tokio::sync::mpsc::channel;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;

#[derive(Debug)]
pub struct Msg {
    inner: Option<Box<dyn DataFrame>>,
}

impl Msg {
    fn new() -> Self {
        Self {
            inner: Some(Box::new(Df {
                col_a: vec![1, 2, 3],
                col_b: vec!["one", "two", "three"],
            })),
        }
    }
}

impl Message for Msg {
    fn ack(&mut self) -> section::message::Ack {
        Box::pin(async {})
    }

    fn next(&mut self) -> section::message::Next<'_> {
        let df = self.inner.take().map(Chunk::DataFrame);
        Box::pin(async move { Ok(df) })
    }

    fn origin(&self) -> &str {
        "example"
    }
}

#[derive(Debug)]
pub struct Df {
    col_a: Vec<u64>,
    col_b: Vec<&'static str>,
}

impl DataFrame for Df {
    fn columns(&self) -> Vec<section::message::Column<'_>> {
        vec![
            Column::new(
                "col_a",
                DataType::U64,
                Box::new(self.col_a.iter().copied().map(ValueView::U64)),
            ),
            Column::new(
                "col_b",
                DataType::Str,
                Box::new(self.col_b.iter().copied().map(ValueView::Str)),
            ),
        ]
    }
}

#[tokio::main]
async fn main() {
    let source = ToCsv::new(2);

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
                tx_in.send(Box::new(Msg::new())).await.unwrap();
            },
            msg = rx_out.recv() => {
                let mut msg = msg.unwrap();
                println!("got msg:\n-----");
                while let Some(chunk) = msg.next().await.unwrap() {
                    match chunk {
                        Chunk::DataFrame(_) => unreachable!(),
                        Chunk::Byte(bin) => {
                            print!("{}", unsafe { String::from_utf8_unchecked(bin) })
                        },
                    };
                }
                println!("-----");
                msg.ack().await;
            }
        }
    }
}
