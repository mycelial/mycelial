
use clap::Parser;
use section::{dummy::DummySectionChannel, prelude::*, pretty_print::pretty_print};
use stub::Stub;
use tokio::sync::mpsc::channel;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;

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
}

#[tokio::main]
async fn main() -> Result<(), SectionError> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let source = s3::S3Source::new(
        cli.bucket,
        cli.region,
        cli.access_key_id,
        cli.secret_key,
        false,
        "",
        5,
    );

    let (tx, rx) = channel::<SectionMessage>(1);
    let tx = PollSender::new(tx).sink_map_err(|_| "send error".into());
    let mut rx = ReceiverStream::new(rx);

    let handle = tokio::spawn(async move {
        source
            .start(
                Stub::<SectionMessage, SectionError>::new(),
                Box::new(tx),
                DummySectionChannel::new(),
            )
            .await
            .unwrap();
    });

    while let Some(mut msg) = rx.next().await {
        while let Some(chunk) = msg.next().await? {
            match chunk {
                Chunk::DataFrame(df) => {
                    println!("{}", pretty_print(&*df));
                }
                Chunk::Byte(_) => return Err("expected dataframe")?,
            }
        }
    }
    handle.await?;
    Ok(())
}
