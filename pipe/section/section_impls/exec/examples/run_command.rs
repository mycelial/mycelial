//! Run use provided command over incoming dataframe
//! Current implementation of `Exec` section assumes execution per row

use clap::Parser;
use exec::Exec;
use section::{
    dummy::DummySectionChannel,
    futures::{self, FutureExt, SinkExt},
    message::{Ack, Chunk, Column, DataFrame, DataType, Message, ValueView},
    pretty_print::pretty_print,
    section::Section,
    SectionMessage,
};
use std::time::Duration;
use tokio::{sync::mpsc::channel, time::Instant};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;

#[derive(Debug, Parser)]
struct Cli {
    #[clap(short, long)]
    command: String,
}

#[derive(Debug)]
struct Msg {
    origin: String,
    df: Option<Box<dyn DataFrame>>,
}

impl Msg {
    fn new(tick: Instant) -> Box<Self> {
        Box::new(Self {
            origin: format!("{}", tick.elapsed().as_millis()),
            df: Some(Box::new(Df {})),
        })
    }
}

#[derive(Debug)]
struct Df;

impl DataFrame for Df {
    fn columns(&self) -> Vec<section::message::Column<'_>> {
        vec![Column::new(
            "key",
            DataType::Str,
            Box::new(std::iter::once(ValueView::Str("value"))),
        )]
    }
}

impl Message for Msg {
    fn ack(&mut self) -> Ack {
        // dummy channel doesn't allow acks anyway
        Box::pin(async {})
    }

    fn next(&mut self) -> section::message::Next<'_> {
        let df = self.df.take().map(Chunk::DataFrame);
        Box::pin(async move { Ok(df) })
    }

    fn origin(&self) -> &str {
        self.origin.as_str()
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let command = cli
        .command
        .splitn(2, ' ')
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>();

    let (command, args) = match *command.as_slice() {
        [command] => (command, None),
        [command, args] => (command, Some(args)),
        _ => unreachable!(),
    };
    let source = Exec::new(command, args, true, false).unwrap();

    let (tx_in, rx_in) = channel::<SectionMessage>(1);
    let rx_in = ReceiverStream::new(rx_in);
    let (tx_out, mut rx_out) = channel::<SectionMessage>(1);
    let tx_out = PollSender::new(tx_out).sink_map_err(|_| "send error".into());

    tokio::spawn(async move {
        source
            .start(rx_in, tx_out, DummySectionChannel::new())
            .await
            .unwrap();
    });
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        futures::select! {
            tick = interval.tick().fuse() => {
                let msg = Msg::new(tick);
                tx_in.send(msg).await.unwrap();
            },
            msg = rx_out.recv().fuse() => {
                let mut msg = msg.unwrap();
                println!("got message: {:?}", msg);
                while let Some(chunk) = msg.next().await.unwrap() {
                    match chunk {
                        Chunk::DataFrame(df) => println!("{}", pretty_print(&*df)),
                        Chunk::Byte(_bin) => println!("bin"),
                    }
                }
                msg.ack().await;
            }
        }
    }
}
