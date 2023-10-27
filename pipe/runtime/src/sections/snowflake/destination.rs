use futures::{Sink, Stream, StreamExt, FutureExt};
use section::{Section, Command, SectionChannel};
use std::pin::pin;
use arrow::datatypes::{DataType, SchemaRef};
use parquet::arrow::AsyncArrowWriter;
use parquet::errors::ParquetError;
use snowflake_api::{SnowflakeApiError, SnowflakeApi};
use tempfile::tempdir;
use thiserror::Error;
use tokio::fs::File;

use crate::{
    config::Map,
    message::Message,
    types::{DynSection, SectionError, SectionFuture},
};

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
    // destination
    table: String,
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
        table: impl Into<String>,
    ) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            role: role.into(),
            account_identifier: account_identifier.into(),
            warehouse: warehouse.into(),
            database: database.into(),
            schema: schema.into(),
            table: table.into(),
        }
    }

    async fn enter_loop<Input, Output, SectionChan>(
        self,
        input: Input,
        _output: Output,
        mut section_chan: SectionChan,
    ) -> Result<(), SectionError>
        where
            Input: Stream<Item=Message>,
            Output: Sink<<Input as Stream>::Item>,
            SectionChan: SectionChannel,
    {
        let mut input = pin!(input.fuse());
        let mut api = SnowflakeApi::with_password_auth(
            &self.account_identifier,
            &self.warehouse,
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
                        return Ok(())
                    }
                    let mut msg = msg.unwrap();
                    self.destructive_load_batch(&mut api, &msg.payload).await?;
                    msg.ack().await;
                }
            }
        }
    }

    async fn destructive_load_batch(&self, api: &mut SnowflakeApi, batch: &arrow::record_batch::RecordBatch) -> Result<(), SnowflakeDestinationError> {
        // fixme: race condition on multiple batches in succession, disambiguate file names?
        let tmp_dir = tempdir()?;
        let file_path = &tmp_dir.path().join("mycelial.parquet");
        log::info!("Dumping batch to parquet file: {}", file_path.to_str().unwrap());
        let mut tmp_file = File::create(file_path).await?;

        let mut writer =
            AsyncArrowWriter::try_new(&mut tmp_file, batch.schema(), 0, None)?;
        writer.write(batch).await?;
        writer.close().await?;

        // todo: use load and select into custom stage
        let table_name = self.table.as_str();
        let schema = self.arrow_schema_to_snowflake_schema(batch.schema());
        api.exec(
            &format!("CREATE TABLE IF NOT EXISTS {}({});", table_name, schema)
        ).await?;

        // fixme: unwrap
        api.exec(
            &format!("PUT file://{} @%{};", &file_path.to_str().unwrap(), table_name)
        ).await?;

        api.exec(
            "CREATE OR REPLACE TEMPORARY FILE FORMAT CUSTOM_PARQUET_FORMAT TYPE = PARQUET COMPRESSION = NONE TRIM_SPACE = TRUE REPLACE_INVALID_CHARACTERS = TRUE BINARY_AS_TEXT = FALSE;"
        ).await?;

        api.exec(
            &format!("TRUNCATE TABLE {};", table_name)
        ).await?;

        api.exec(
            &format!("COPY INTO {} FILE_FORMAT = CUSTOM_PARQUET_FORMAT PURGE = TRUE MATCH_BY_COLUMN_NAME = CASE_INSENSITIVE;", table_name)
        ).await?;

        Ok(())
    }

    // https://github.com/apache/parquet-format/blob/master/LogicalTypes.md
    // Since type conversion goes Arrow -> Parquet -> Snowflake
    // The Arrow schema mapping must match what Snowflake expects on load instead of being
    // logically mapped directly from Arrow types
    fn arrow_schema_to_snowflake_schema(&self, arrow_schema: SchemaRef) -> String {
        arrow_schema.fields.iter().map(|f| {
            // todo: use Parquet directly
            let snowflake_type = match f.data_type() {
                DataType::Boolean => "BOOLEAN",
                // null encoded as int32 in parquet
                DataType::Null |
                // time32 and time64 denote time of day and encoded as int in parquet too
                DataType::Time32(_) | DataType::Time64(_) |
                // rest of ints
                DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64 | DataType::UInt8 | DataType::UInt16 | DataType::UInt32 | DataType::UInt64 => "NUMBER",
                DataType::Float16 | DataType::Float32 | DataType::Float64 | DataType::Decimal128(_, _) | DataType::Decimal256(_, _) => "FLOAT",
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
        Input: Stream<Item=Message> + Send + 'static,
        Output: Sink<<Input as Stream>::Item> + Send + 'static,
        SectionChan: SectionChannel + Send + 'static,
{
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(self, input: Input, output: Output, section_chan: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, section_chan).await })
    }
}

pub fn constructor<S: SectionChannel>(config: &Map) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let username = config
        .get("username")
        .ok_or("username required")?
        .as_str()
        .ok_or("'username' should be a string")?;
    let password = config
        .get("password")
        .ok_or("password required")?
        .as_str()
        .ok_or("'password' should be a string")?;
    let role = config
        .get("role")
        .ok_or("role required")?
        .as_str()
        .ok_or("'role' should be a string")?;
    let account_identifier = config
        .get("account_identifier")
        .ok_or("account_identifier required")?
        .as_str()
        .ok_or("'account_identifier' should be a string")?;
    let warehouse = config
        .get("warehouse")
        .ok_or("warehouse required")?
        .as_str()
        .ok_or("'warehouse' should be a string")?;
    let database = config
        .get("database")
        .ok_or("database required")?
        .as_str()
        .ok_or("'database' should be a string")?;
    let schema = config
        .get("schema")
        .ok_or("schema required")?
        .as_str()
        .ok_or("'schema' should be a string")?;
    let query = config
        .get("query")
        .ok_or("query required")?
        .as_str()
        .ok_or("'query' should be a string")?;
    Ok(Box::new(SnowflakeDestination::new(
            username,
            password,
            role,
            account_identifier,
            warehouse,
            database,
            schema,
            query,
        )
    ))
}
