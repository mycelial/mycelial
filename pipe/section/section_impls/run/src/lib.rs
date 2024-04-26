//! Run section
//!
//! Run section in current implementation allows to run and supervices long running-external programs
//! Interface is very similar to exec section, but for now section doesn't accept any input or produce
//! any output.
//! Stdout/Stderr of external program is captured and logged

use std::pin::pin;
use std::process::Stdio;

use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt},
    section::Section,
    SectionError, SectionFuture,
};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader, Lines};

pub type Result<T, E = SectionError> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct Run {
    command: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
}

impl Run {
    pub fn new(command: &str, args: Option<&str>, env: &[(&str, &str)]) -> Result<Self> {
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
            env,
        })
    }
}

fn to_line_reader<R: AsyncRead>(reader: R) -> Lines<BufReader<R>> {
    BufReader::new(reader).lines()
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Run
where
    Input: Send + 'static,
    Output: Send + 'static,
    SectionChan: SectionChannel + Send + 'static,
{
    type Future = SectionFuture;
    type Error = SectionError;

    fn start(
        self,
        _input: Input,
        _output: Output,
        mut section_channel: SectionChan,
    ) -> Self::Future {
        Box::pin(async move {
            let mut command = tokio::process::Command::new(&self.command);
            let envs = self.env.iter().map(|(k, v)| (k.as_str(), v.as_str()));
            command
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .envs(envs)
                .args(self.args.iter());
            let mut child = command.spawn()?;
            let mut child_stdout = pin!(to_line_reader(child.stdout.take().unwrap()));
            let mut child_stderr = pin!(to_line_reader(child.stderr.take().unwrap()));
            loop {
                futures::select! {
                    cmd = section_channel.recv().fuse() => {
                        if let Command::Stop = cmd? {
                            break;
                        }
                    },
                    line = child_stdout.next_line().fuse() => {
                        match line {
                            Ok(Some(line)) => tracing::info!("stdout: {line}"),
                            Ok(None) => {
                                tracing::info!("stdout closed");
                                break;
                            },
                            Err(e) => {
                                tracing::error!("error reading from stdout: {e:?}");
                                Err("stdout closed")?
                            },

                        };
                    },
                    line = child_stderr.next_line().fuse() => {
                        match line {
                            Ok(Some(line)) => tracing::info!("stderr: {line}]"),
                            Ok(None) => {
                                tracing::info!("stderr closed");
                                break;
                            },
                            Err(e) => tracing::error!("error reading from stderr: {e:?}"),
                        };
                    },
                }
            }
            match child.wait().await {
                Ok(exit_status) => {
                    tracing::info!("child exited with {exit_status:?}");
                    Ok(())
                }
                Err(e) => {
                    tracing::error!("error while waiting on child: {e}");
                    Err(e)?
                }
            }
        })
    }
}
