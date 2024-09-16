use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::Parser;
use section::{dummy::DummySectionChannel, prelude::*};
use stub::Stub;
use tokio::sync::mpsc::channel;
use tokio_stream::wrappers::ReceiverStream;

#[derive(Parser)]
struct Cli {
    #[clap(short, long, env = "BUCKET")]
    bucket: String,
    #[clap(short, long, env = "REGION")]
    region: String,
    #[clap(short, long, env = "KEY_ID")]
    access_key_id: String,
    #[clap(short, long, env = "SECRET_KEY")]
    secret_key: String,
    #[clap(short, long, default_value = "4096")]
    max_chunk_size: usize,
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

#[derive(Debug)]
struct Msg {
    origin: String,
    payload: Vec<Vec<u8>>,
}

impl Msg {
    fn new(max_chunk_size: usize) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        let mut rng = XorShift::new(now);
        let len = ((rng.next() as usize) % 15) + 1;
        let payload: Vec<Vec<u8>> = (0..len)
            .map(|_| {
                static BYTES: &[u8] = &[b'a', b'b', b'c', b'd'];
                let len = ((rng.next() as usize) % max_chunk_size) + 1;
                (0..len)
                    .map(|_| BYTES[rng.next() as usize % BYTES.len()])
                    .collect()
            })
            .collect();
        tracing::info!(
            "putting {} byte object at {}",
            payload.iter().map(|chunk| chunk.len()).sum::<usize>(),
            now
        );
        Self {
            origin: format!("{now}"),
            payload,
        }
    }
}

impl Message for Msg {
    fn ack(&mut self) -> section::message::Ack {
        Box::pin(async move {})
    }

    fn next(&mut self) -> section::message::Next<'_> {
        let payload = Ok(self.payload.pop().map(Chunk::Byte));
        Box::pin(async move { payload })
    }

    fn origin(&self) -> &str {
        self.origin.as_str()
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let destination = s3::S3Destination::new(
        cli.bucket,
        cli.region,
        cli.access_key_id,
        cli.secret_key,
        1 << 23,
    );

    let (tx, rx) = channel::<SectionMessage>(1);
    let rx = ReceiverStream::new(rx);
    let mut interval = tokio::time::interval(Duration::from_secs(5));

    tokio::spawn(async move {
        destination
            .start(
                Box::new(rx),
                Stub::<SectionMessage, SectionError>::new(),
                DummySectionChannel::new(),
            )
            .await
            .unwrap();
    });

    loop {
        tokio::select! {
            _ = interval.tick() => {
                tx.send(Box::new(Msg::new(cli.max_chunk_size))).await.unwrap();
            },
        }
    }
}
