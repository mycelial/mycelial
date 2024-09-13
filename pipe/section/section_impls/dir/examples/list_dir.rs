//! List directory with applied filters
//! This example outputs dataframe with full paths

use clap::Parser;
use dir::DirSource;
use section::{
    dummy::DummySectionChannel, futures::SinkExt, message::Chunk, pretty_print::pretty_print,
    section::Section, SectionError, SectionMessage,
};
use stub::Stub;
use tokio::sync::mpsc::channel;
use tokio_util::sync::PollSender;

#[derive(Debug, Parser)]
struct Cli {
    #[clap(short, long)]
    dir_path: String,
    #[clap(short, long, default_value = "")]
    pattern: String,
    #[clap(short, long, default_value = "")]
    start_after: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let source = DirSource::new(cli.dir_path, cli.pattern, cli.start_after, 3, false);

    let (tx, mut rx) = channel(1);
    let tx = PollSender::new(tx).sink_map_err(|_| "send error".into());

    tokio::spawn(async move {
        source
            .start(
                Stub::<SectionMessage, SectionError>::new(),
                Box::new(tx),
                DummySectionChannel::new(),
            )
            .await
            .unwrap();
    });
    while let Some(mut msg) = rx.recv().await {
        println!("message: {msg:?}");
        while let Some(chunk) = msg.next().await.unwrap() {
            match chunk {
                Chunk::DataFrame(df) => println!("{}", pretty_print(&*df)),
                Chunk::Byte(_bin) => println!("bin"),
            };
        }
        msg.ack().await;
    }
}
