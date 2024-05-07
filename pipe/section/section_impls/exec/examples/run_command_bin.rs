//! Run use provided command over incoming binary stream
//! Current implementation of `Exec` section assumes execution per row

use clap::Parser;
use exec::ExecBin as Exec;
use section::{
    dummy::DummySectionChannel,
    futures::{self, FutureExt, SinkExt},
    message::{Ack, Chunk, Message},
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
    payload: Vec<u8>,
}

impl Msg {
    fn new(tick: Instant) -> Box<Self> {
        let origin = format!("{}", tick.elapsed().as_millis());
        Box::new(Self {
            payload: "hello world".as_bytes().iter().copied().rev().collect(),
            origin,
        })
    }
}

impl Message for Msg {
    fn ack(&mut self) -> Ack {
        // dummy channel doesn't allow acks anyway
        Box::pin(async {})
    }

    fn next(&mut self) -> section::message::Next<'_> {
        let last = self.payload.pop().map(|byte| Chunk::Byte(vec![byte]));
        Box::pin(async move { Ok(last) })
    }

    fn origin(&self) -> &str {
        self.origin.as_str()
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
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
    let source = Exec::new(command, args, false, &[]).unwrap();

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
                println!("output >>>");
                while let Some(chunk) = msg.next().await.unwrap() {
                    match chunk {
                        Chunk::DataFrame(_) => unreachable!(),
                        Chunk::Byte(bin) => print!("{}", String::from_utf8_lossy(&bin)),
                    }
                }
                println!();
                println!("<<<");
                msg.ack().await;
            }
        }
    }
}
