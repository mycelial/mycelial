//! Reshift data loader
//! Utilizes COPY operation from S3 buckets
//!
//! Section expects dataframe with column 'path', where 'path' is a full path to object in s3
//! bucket

use std::{
    pin::pin,
    str::FromStr,
    time::{Duration, Instant},
};

use section::prelude::*;
use sqlx::{postgres::PgConnectOptions, ConnectOptions};

use crate::RedshiftLoader;

/// Escape value
pub fn escape(name: impl AsRef<str>, symbol: char) -> String {
    name.as_ref()
        .chars()
        .flat_map(|char| {
            let maybe_char = match char == symbol {
                true => Some('\\'),
                false => None,
            };
            maybe_char.into_iter().chain([char])
        })
        .collect::<String>()
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for RedshiftLoader
where
    Input: SectionStream,
    Output: SectionSink,
    SectionChan: SectionChannel,
{
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(
        self,
        input: Input,
        _output: Output,
        mut section_channel: SectionChan,
    ) -> Self::Future {
        Box::pin(async move {
            let mut input = pin!(input);
            let database_url = format!(
                "postgres://{}:{}@{}:{}/{}",
                self.user, self.password, self.host, self.port, self.database
            );
            let mut connection = PgConnectOptions::from_str(database_url.as_str())?
                .extra_float_digits(2)
                .log_slow_statements(log::LevelFilter::Debug, Duration::from_secs(1))
                .connect()
                .await?;

            let data_format = match self.data_format.to_uppercase().as_str() {
                "CSV" => "CSV",
                other => Err(format!("unsupported data format: {other}"))?,
            };

            loop {
                futures::select! {
                    cmd = section_channel.recv().fuse() => {
                        if let Command::Stop = cmd? {
                            return Ok(())
                        }
                    },
                    msg = input.next().fuse() => {
                        let mut msg = match msg {
                            Some(msg) => msg,
                            None => Err("input closed")?
                        };
                        while let Some(chunk) = msg.next().await? {
                            let df = match chunk {
                                Chunk::DataFrame(df) => df,
                                _ => Err("expected dafaframe stream")?
                            };
                            let paths = match df.columns().into_iter().find(|col| col.name() == "path") {
                                Some(col) => col,
                                None => Err("expected to have field 'path' with s3 objects paths in dataframe")?
                            };
                            for path in paths {
                                let path = match path {
                                    ValueView::Str(path) => path,
                                    _ => Err("expected path as a string value")?
                                };
                                let query = format!(
                                    "COPY \"{}\" FROM '{}' iam_role '{}' region '{}' {data_format} {}",
                                    escape(msg.origin(), '"'),
                                    escape(path, '\''),
                                    escape(self.iam_role.as_str(), '\''),
                                    escape(self.region.as_str(), '\''),
                                    if self.ignore_header { "IGNOREHEADER 1" } else { "" }
                                );
                                let start = Instant::now();
                                sqlx::query(&query).execute(&mut connection).await?;
                                tracing::debug!("took {}ms to load {}", start.elapsed().as_millis(), path);
                            }
                        }
                        msg.ack().await;
                    }
                }
            }
        })
    }
}
