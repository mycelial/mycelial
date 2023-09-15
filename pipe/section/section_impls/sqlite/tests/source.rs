use futures::{StreamExt, SinkExt};
use stub::Stub;
use section::{RootChannel as _, SectionChannel as _, Section as _};
use runtime::{
    command_channel::RootChannel,
    channel::{self, channel},
};
use sqlite::{source, Message};
use tempfile::NamedTempFile;


type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[tokio::test]
async fn source() -> Result<(), StdError> {
    let db_path = NamedTempFile::new()?.path().to_string_lossy().to_string();
    let mut root_chan = RootChannel::new();
    let section_chan = root_chan.section_channel(0)?;

    let sqlite_source = source::Sqlite::new(db_path, &["*"]);
    let (output, rx) = channel::channel(1);
    //let output = output.sink_map_err(|e| e.into());

    let section = sqlite_source.start(
        rx, //Stub::new(),
        output,
        section_chan
    );
 // while let Some(out) = rx.next().await {
 //     println!("got: {:?}", out)
 // }
    Ok(())
}
