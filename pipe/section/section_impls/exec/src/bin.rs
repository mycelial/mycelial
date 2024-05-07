//! ExecBin section
//! Section executes user provided script and pipes incoming binary stream into spawned process stdin,
//! stdout of spawned process becomes new output stream
//! Message acknowledgement can be delegated to downstream, if value of `ack_passthrough` set to True.

use std::{
    pin::pin,
    process::{ExitStatus, Stdio},
};
use tokio::{
    io::{
        AsyncBufReadExt, AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _, BufReader,
    },
    process::Child,
    sync::mpsc::{channel, Receiver, Sender},
};

use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt, Sink, SinkExt, Stream, StreamExt},
    message::{Ack, Chunk, Message},
    section::Section,
    SectionError, SectionFuture, SectionMessage,
};

type Result<T, E = SectionError> = std::result::Result<T, E>;
struct Msg {
    origin: String,
    rx: Receiver<Result<Option<Chunk>>>,
    ack: Option<Ack>,
}

impl std::fmt::Debug for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Msg")
            .field("origin", &self.origin)
            .field(
                "ack",
                &self.ack.as_ref().map(|_| "Some<Ack>").unwrap_or("None"),
            )
            .finish()
    }
}

impl Message for Msg {
    fn ack(&mut self) -> Ack {
        self.ack.take().unwrap_or(Box::pin(async {}))
    }

    fn next(&mut self) -> section::message::Next<'_> {
        Box::pin(async {
            match self.rx.recv().await {
                Some(msg) => msg,
                None => Err("closed".into()),
            }
        })
    }

    fn origin(&self) -> &str {
        self.origin.as_str()
    }
}

#[derive(Debug)]
pub struct ExecBin {
    // command to rnu
    command: String,
    // command arguments
    args: Vec<String>,
    // passthrough ack into downstream
    ack_passthrough: bool,
    // command env
    env: Vec<(String, String)>,
}

impl ExecBin {
    pub fn new(
        command: &str,
        args: Option<&str>,
        ack_passthrough: bool,
        env: &[(&str, &str)],
    ) -> Result<Self> {
        if command.is_empty() {
            Err("empty commands are not allowed")?
        }
        let env = env
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let args = shlex::split(args.unwrap_or("")).unwrap_or_default();
        Ok(Self {
            command: command.into(),
            args,
            ack_passthrough,
            env,
        })
    }

    fn run_command(&self) -> Result<Child> {
        let mut command = tokio::process::Command::new(&self.command);
        let envs = self.env.iter().map(|(k, v)| (k.as_str(), v.as_str()));
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .envs(envs)
            .args(self.args.iter());
        Ok(command.spawn()?)
    }
}

async fn stream_to_stdin(
    mut msg: SectionMessage,
    mut writer: impl AsyncWrite + Unpin,
) -> Result<()> {
    while let Some(chunk) = msg.next().await? {
        match chunk {
            Chunk::Byte(b) => writer.write_all(&b).await?,
            Chunk::DataFrame(_) => Err("Exec binary doesn't work with dataframe stream")?,
        };
    }
    Ok(())
}

async fn stdout_to_msgstream(
    mut reader: impl AsyncRead + Unpin,
    tx: Sender<Result<Option<Chunk>>>,
) -> Result<()> {
    let mut buf = vec![0; 4096];
    loop {
        let read = reader.read(&mut buf).await?;
        if read == 0 {
            // stdout closed
            break;
        };
        unsafe {
            buf.set_len(read);
        };
        let mut out_buf = vec![0; 4096];
        std::mem::swap(&mut buf, &mut out_buf);
        tx.send(Ok(Some(Chunk::Byte(out_buf))))
            .await
            .map_err(|_| "output message channel closed")?;
    }
    tx.send(Ok(None))
        .await
        .map_err(|_| "output message channel closed")?;
    Ok(())
}

async fn stderr_to_log(reader: impl AsyncRead + Unpin) -> Result<()> {
    let mut line_reader = BufReader::new(reader).lines();
    while let Some(line) = line_reader.next_line().await? {
        tracing::info!("stderr: {line}");
    }
    Ok(())
}

async fn wait_child(mut child: Child) -> Result<ExitStatus> {
    Ok(child.wait().await?)
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for ExecBin
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel + Send + 'static,
{
    type Future = SectionFuture;
    type Error = SectionError;

    fn start(self, input: Input, output: Output, mut section_channel: SectionChan) -> Self::Future {
        Box::pin(async move {
            let mut input = pin!(input);
            let mut output = pin!(output);
            loop {
                futures::select! {
                    msg = input.next().fuse() => {
                        let mut msg = match msg {
                            Some(msg) => msg,
                            None => Err("input closed")?,
                        };

                        let mut ack = Some(msg.ack());
                        let (tx, rx) = channel(1);
                        let downstream_msg = Msg {
                            origin: msg.origin().to_string(),
                            rx,
                            ack: if self.ack_passthrough { ack.take()}  else { None }
                        };
                        output.send(Box::new(downstream_msg)).await?;
                        let mut child = self.run_command()?;
                        let stdin = child.stdin.take().unwrap();
                        let stdout = child.stdout.take().unwrap();
                        let stderr = child.stderr.take().unwrap();

                        let (_, _, _, exit_status) = futures::try_join!(
                            stream_to_stdin(msg, stdin),
                            stdout_to_msgstream(stdout, tx),
                            stderr_to_log(stderr),
                            wait_child(child),
                        )?;
                        if !exit_status.success() {
                            Err(format!("child failed with exit code: {exit_status:?}"))?;
                        }
                        if let Some(ack) = ack {
                            ack.await;
                        }
                    }
                    cmd = section_channel.recv().fuse() => {
                        if let Command::Stop = cmd? {
                            return Ok(())
                        }
                    }
                }
            }
        })
    }
}
