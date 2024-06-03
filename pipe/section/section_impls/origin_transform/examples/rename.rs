//! List directory with applied filters
//! This example outputs dataframe with full paths

use origin_transform::OriginTransform;
use section::{
    dummy::DummySectionChannel,
    futures::{self, FutureExt, SinkExt},
    message::{Ack, Message},
    section::Section,
    SectionMessage,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::channel;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;

#[derive(Debug)]
struct Msg {
    origin: String,
}

struct XorShift {
    state: u64,
}

impl XorShift {
    fn new(state: u64) -> Self {
        Self {
            state: state.max(1),
        }
    }

    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        self.state
    }
}

impl Msg {
    fn new() -> Box<Self> {
        static VALUES: &[char] = &[
            'a', 'b', 'c', 'd', 'e', 'f', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
        ];
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut rng = XorShift::new(now);

        let depth = (rng.next() % 5) + 1;
        let origin = (0..depth)
            .map(|_| {
                let len = (rng.next() % 5) + 1;
                (0..len)
                    .map(|_| VALUES[rng.next() as usize % VALUES.len()])
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("/");
        let origin = match rng.next() % 2 {
            0 => format!("/{origin}"),
            _ => origin,
        };
        Box::new(Self { origin })
    }
}

impl Message for Msg {
    fn ack(&mut self) -> Ack {
        Box::pin(async {})
    }

    fn next(&mut self) -> section::message::Next<'_> {
        Box::pin(async move { Ok(None) })
    }

    fn origin(&self) -> &str {
        self.origin.as_str()
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let transform = OriginTransform::new("/?.*/", "/tmp/").unwrap();

    let (tx_in, rx_in) = channel::<SectionMessage>(1);
    let rx_in = ReceiverStream::new(rx_in);
    let (tx_out, mut rx_out) = channel::<SectionMessage>(1);
    let tx_out = PollSender::new(tx_out).sink_map_err(|_| "send error".into());

    tokio::spawn(async move {
        transform
            .start(
                Box::new(rx_in),
                Box::new(tx_out),
                DummySectionChannel::new(),
            )
            .await
            .unwrap();
    });
    let mut interval = tokio::time::interval(Duration::from_secs(3));

    loop {
        futures::select! {
            _tick = interval.tick().fuse() => {
                let msg = Msg::new();
                println!("origin: {}", msg.origin());
                tx_in.send(msg).await.unwrap();
            },
            msg = rx_out.recv().fuse() => {
                let msg = msg.unwrap();
                println!("new origin: {}", msg.origin());
            }
        }
    }
}
