//! Exec section
//! Executes user provided script for each row in incoming dataframe.
//!
//! Incoming row can be expanded into script arguments if value of `row_as_args` is set to true.
//! Example:
//! For given dataframe, script will receive 2 extra arguments as --col=val --col1=val1
//! +------------+
//! | col | col1 |
//! +-----+------+
//! | val | val1 |
//! +-----+------+
//!
//! Message acknowledgement can be delegated to downstream, if value of `ack_passthrough` set to True.
//!
//! NOTE: Binary streams are not yet supported.
//! Section doesn't alter incoming message in any way and delivers it in downstream in the same shape and form.

use std::{ffi::OsString, os::unix::ffi::OsStringExt, pin::pin, process::Stdio};
use tokio::sync::mpsc::{channel, Receiver};

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
                &if self.ack.is_some() {
                    "Some<Ack>"
                } else {
                    "None"
                },
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
pub struct Exec {
    command: String,
    args: Vec<String>,
    // convert dataframe rows to command input params
    row_as_args: bool,
    // passthrough ack into downstream
    ack_passthrough: bool,
}

impl Exec {
    pub fn new(
        command: &str,
        args: Option<&str>,
        row_as_args: bool,
        ack_passthrough: bool,
    ) -> Result<Self> {
        if command.is_empty() {
            Err("empty commands are not allowed")?
        }
        let args = args
            .unwrap_or("")
            .split(' ')
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
            .collect::<Vec<_>>();
        Ok(Self {
            command: command.into(),
            args,
            row_as_args,
            ack_passthrough,
        })
    }

    async fn run_command<'a>(&self, args: Vec<String>) -> Result<()> {
        let mut command = tokio::process::Command::new(&self.command);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .args(self.args.iter());
        if self.row_as_args {
            command.args(args);
        }
        let output = command.spawn()?.wait_with_output().await?;
        match output.status.success() {
            true => {
                let os_string: OsString = OsString::from_vec(output.stdout);
                tracing::debug!("successful exec: {}", os_string.to_string_lossy());
                Ok(())
            }
            false => Err(OsString::from_vec(output.stderr)
                .to_string_lossy()
                .to_string())?,
        }
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Exec
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

                        while let Some(chunk) = msg.next().await? {
                            match chunk {
                                Chunk::DataFrame(df) => {
                                    {
                                        let mut columns = df.columns();
                                        'outer: loop {
                                            let mut args = vec![];
                                            for col in columns.iter_mut() {
                                                match col.next() {
                                                    Some(value) => {
                                                        args.push(format!("--{}", col.name()));
                                                        args.push(value.to_string());
                                                    },
                                                    None => break 'outer,
                                                };
                                            }
                                            self.run_command(args).await?;
                                        }
                                    }
                                    tx.send(Ok(Some(Chunk::DataFrame(df)))).await.map_err(|_| "send error")?;
                                },
                                Chunk::Byte(_) => {
                                    Err("byte streams are not yet supported")?;
                                },
                            }
                        }
                        tx.send(Ok(None)).await.map_err(|_| "send error")?;
                        if let Some(ack) = ack.take() {
                            ack.await;
                        }
                    },
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
