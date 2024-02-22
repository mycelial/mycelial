use arrow_msg::{
    arrow::{
        datatypes::{DataType, SchemaRef},
        record_batch::RecordBatch,
    },
    df_to_recordbatch,
};
use parquet::arrow::AsyncArrowWriter;
use parquet::errors::ParquetError;
use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt, Sink, Stream, StreamExt},
    message::Chunk,
    section::Section,
    SectionError, SectionFuture, SectionMessage,
};
use snowflake_api::{SnowflakeApi, SnowflakeApiError};
use std::pin::pin;
use tempfile::tempdir;
use thiserror::Error;
use tokio::fs::File;

#[derive(Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum SnowflakeDestinationError {
    #[error(transparent)]
    RequestError(#[from] SnowflakeApiError),

    #[error(transparent)]
    ParquetError(#[from] ParquetError),

    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

pub struct SnowflakeDestination {
    username: String,
    password: String,
    role: String,
    account_identifier: String,
    warehouse: String,
    database: String,
    schema: String,
    truncate: bool,
}

impl SnowflakeDestination {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
        role: impl Into<String>,
        account_identifier: impl Into<String>,
        warehouse: impl Into<String>,
        database: impl Into<String>,
        schema: impl Into<String>,
        truncate: bool,
    ) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            role: role.into(),
            account_identifier: account_identifier.into(),
            warehouse: warehouse.into(),
            database: database.into(),
            schema: schema.into(),
            truncate,
        }
    }

    async fn enter_loop<Input, Output, SectionChan>(
        self,
        input: Input,
        _output: Output,
        mut section_chan: SectionChan,
    ) -> Result<(), SectionError>
    where
        Input: Stream<Item = SectionMessage>,
        Output: Sink<<Input as Stream>::Item>,
        SectionChan: SectionChannel,
    {
        let mut input = pin!(input.fuse());
        let mut api = SnowflakeApi::with_password_auth(
            &self.account_identifier,
            Some(&self.warehouse),
            Some(&self.database),
            Some(&self.schema),
            &self.username,
            Some(&self.role),
            &self.password,
        )?;

        loop {
            futures::select! {
                cmd = section_chan.recv().fuse() => {
                    if let Command::Stop = cmd? {
                        return Ok(())
                    }
                },
                msg = input.next() => {
                    if msg.is_none() {
                        Err("stream closed")?
                    }
                    let mut msg = msg.unwrap();
                    while let Some(chunk) = msg.next().await? {
                        let batch = match chunk {
                            Chunk::DataFrame(df) => {
                                for column in df.columns() {
                                    if column.data_type() == section::message::DataType::Any {
                                        Err(format!("snowflake destination can't handle column '{}' with DataType::Any", column.name()))?
                                    }
                                }
                                df_to_recordbatch(df.as_ref())?
                            },
                            _ => Err(format!("unsupported chunk: {:?}", chunk))?,
                        };
                        self.destructive_load_batch(&mut api, &batch, msg.origin()).await?;
                    }
                    msg.ack().await;
                }
            }
        }
    }

    async fn destructive_load_batch(
        &self,
        api: &mut SnowflakeApi,
        batch: &RecordBatch,
        origin: &str,
    ) -> Result<(), SnowflakeDestinationError> {
        // fixme: race condition on multiple batches in succession, disambiguate file names?
        let tmp_dir = tempdir()?;
        let file_path = &tmp_dir.path().join("mycelial.parquet");
        let mut tmp_file = File::create(file_path).await?;

        let mut writer = AsyncArrowWriter::try_new(&mut tmp_file, batch.schema(), 0, None)?;
        writer.write(batch).await?;
        writer.close().await?;

        // todo: use load and select into custom stage

        // todo: this table name substitution is not smart.
        let table_name = origin.replace([' ', '/', ':', '.'], "_");
        let schema = self.arrow_schema_to_snowflake_schema(batch.schema());
        api.exec(&format!(
            "CREATE TABLE IF NOT EXISTS {}({});",
            table_name, schema
        ))
        .await?;

        // fixme: unwrap
        api.exec(&format!(
            "PUT file://{} @%{};",
            &file_path.to_str().unwrap(),
            table_name
        ))
        .await?;

        api.exec(
            "CREATE OR REPLACE TEMPORARY FILE FORMAT CUSTOM_PARQUET_FORMAT TYPE = PARQUET COMPRESSION = AUTO TRIM_SPACE = TRUE REPLACE_INVALID_CHARACTERS = TRUE BINARY_AS_TEXT = FALSE USE_LOGICAL_TYPE = TRUE;"
        ).await?;

        if self.truncate {
            api.exec(&format!("TRUNCATE TABLE {};", table_name)).await?;
        }

        api.exec(&format!(
            "COPY INTO {table_name} FILE_FORMAT = CUSTOM_PARQUET_FORMAT PURGE = TRUE MATCH_BY_COLUMN_NAME = CASE_INSENSITIVE;"
        )).await?;

        Ok(())
    }

    // https://github.com/apache/parquet-format/blob/master/LogicalTypes.md
    // Since type conversion goes Arrow -> Parquet -> Snowflake
    // The Arrow schema mapping must match what Snowflake expects on load instead of being
    // logically mapped directly from Arrow types
    // todo: use Parquet directly
    fn arrow_schema_to_snowflake_schema(&self, arrow_schema: SchemaRef) -> String {
        arrow_schema.fields.iter().map(|f| {
            let tmp: String;
            let snowflake_type = match f.data_type() {
                DataType::Boolean => "BOOLEAN",
                DataType::Time32(_) | DataType::Time64(_) => "TIME",
                // null encoded as int32 in parquet
                DataType::Null |
                // rest of ints
                DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64 | DataType::UInt8 | DataType::UInt16 | DataType::UInt32 | DataType::UInt64 => "NUMBER",
                DataType::Float16 | DataType::Float32 | DataType::Float64 => "Float",
                DataType::Decimal128(_, scale) | DataType::Decimal256(_, scale) => {
                    tmp = format!("NUMBER({}, {scale})", 38 - scale); &tmp
                },
                DataType::Date32 | DataType::Date64 => "DATE",
                DataType::Timestamp(_, _) => "TIMESTAMP",
                DataType::Binary | DataType::FixedSizeBinary(_) | DataType::LargeBinary => "BINARY",
                DataType::Utf8 | DataType::LargeUtf8 => "VARCHAR",
                // interval encoded as fixed len byte array
                DataType::Interval(_) |
                // rest of list types
                DataType::List(_) | DataType::FixedSizeList(_, _) | DataType::LargeList(_) | DataType::RunEndEncoded(_, _) => "ARRAY",
                DataType::Struct(_) | DataType::Dictionary(_, _) | DataType::Map(_, _) => "OBJECT",
                DataType::Union(_, _) => "VARIANT",
                DataType::Duration(_)  => unimplemented!(),
            };

            format!("{} {}", f.name(), snowflake_type)
        }).collect::<Vec<String>>().join(", ")
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for SnowflakeDestination
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<<Input as Stream>::Item> + Send + 'static,
    SectionChan: SectionChannel + Send + 'static,
{
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(self, input: Input, output: Output, section_chan: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, section_chan).await })
    }
}
