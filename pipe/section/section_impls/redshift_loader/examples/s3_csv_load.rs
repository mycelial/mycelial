use clap::Parser;
use section::{dummy::DummySectionChannel, prelude::*};
use stub::Stub;
use tokio::sync::mpsc::channel;
use tokio_stream::wrappers::ReceiverStream;

#[derive(Parser)]
struct Cli {
    #[clap(short, long, env = "DATABASE_URL")]
    database_url: String,
    #[clap(short, long, env = "OBJECT")]
    object: String,
    #[clap(short, long, env = "REGION")]
    region: String,
    #[clap(short, long, env = "IAM_ROLE")]
    iam_role: String,
    #[clap(long, env = "ORIGIN")]
    origin: String,
}

#[derive(Debug)]
struct Df {
    path: String,
}

impl DataFrame for Df {
    fn columns(&self) -> Vec<Column<'_>> {
        vec![Column::new(
            "path",
            DataType::Str,
            Box::new(std::iter::once(self.path.as_str().into())),
        )]
    }
}

#[derive(Debug)]
struct Msg {
    origin: String,
    inner: Option<Box<dyn DataFrame>>,
}

impl Message for Msg {
    fn ack(&mut self) -> Ack {
        Box::pin(async {})
    }

    fn next(&mut self) -> Next<'_> {
        let chunk = self.inner.take().map(Chunk::DataFrame);
        Box::pin(async move { Ok(chunk) })
    }

    fn origin(&self) -> &str {
        self.origin.as_str()
    }
}

#[tokio::main]
async fn main() -> Result<(), SectionError> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let redshift_loader = redshift_loader::RedshiftLoader::new(
        &cli.database_url,
        &cli.iam_role,
        &cli.region,
        "CSV",
        true,
    );

    let (tx, rx) = channel::<SectionMessage>(1);
    let rx = ReceiverStream::new(rx);

    let handle = tokio::spawn(async move {
        redshift_loader
            .start(
                Box::new(rx),
                Stub::<SectionMessage, SectionError>::new(),
                DummySectionChannel::new(),
            )
            .await
    });

    let msg = Box::new(Msg {
        origin: cli.origin,
        inner: Some(Box::new(Df { path: cli.object })),
    });
    tx.send(msg).await.unwrap();
    drop(tx);
    handle.await??;
    Ok(())
}
