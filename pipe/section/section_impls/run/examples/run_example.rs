use anyhow::Result;
use run::Run;
use section::{dummy::DummySectionChannel, section::Section, SectionError};
use stub::Stub;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let mut args = std::env::args().skip(1).rev().collect::<Vec<_>>();
    if args.len() < 1 {
        Err(anyhow::anyhow!("example expects at least one argument"))?;
    };
    let command = args.pop().unwrap();
    let args = args.into_iter().rev().collect::<Vec<_>>().join(" ");
    println!("args: {args}");

    let run_source = Run::new(&command, Some(&args), &[]).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("run_source: {:?}", run_source);
    run_source
        .start(
            Stub::<(), SectionError>::new(),
            Stub::<(), SectionError>::new(),
            DummySectionChannel::new(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}
