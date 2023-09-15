//! Dynamic section implementation for Sqlite destination
//!
//! FIXME:
//! 1. Output is not used
use std::{pin::{pin, Pin},  sync::Arc};
use arrow::{
    array::AsArray,
    datatypes::{DataType, Int8Type, Int16Type, Int32Type, Int64Type, Float16Type, Float32Type, Float64Type},
};
use futures::{Stream, StreamExt, Sink};
use crate::{
    Section,
    dynamic_pipe::{
        config::Map,
        message::Message,
        section::{self, DynSection},
    },
    section_impls::sqlite::{Value, generate_schema, SqliteRecordBatch},
    control_channel::SectionChannel
};

use std::future::Future;
use std::str::FromStr;
use sqlx::Connection;
use sqlx::{
    ConnectOptions,
    sqlite::SqliteConnectOptions
};
use crate::section_impls::sqlite::escape_table_name;

#[derive(Debug)]
pub struct Sqlite{
    path: String,
}

impl Sqlite {
    pub fn new(path: impl Into<String>) -> Self {
        Self{ path: path.into() }
    }

    async fn enter_loop<Input, Output>(self, input: Input, output: Output, _command: SectionChannel) -> Result<(), section::Error>
        where Input: Stream<Item=Message> + Send + 'static,
              Output: Sink<Message, Error=section::Error> + Send + 'static,
    {
        let mut input = pin!(input.fuse());
        let mut _output = pin!(output);

        let connection = &mut SqliteConnectOptions::from_str(self.path.as_str())?
            .create_if_missing(true)
            .connect()
            .await?;

        while let Some(mut message) = input.next().await {
            // FIXME: this happens on every incoming batch
            let batch: SqliteRecordBatch = (&message).try_into()?;
            let name = escape_table_name(&batch.name);
            let schema = generate_schema(&batch);
            sqlx::query(&schema).execute(&mut *connection).await?;
            let values_placeholder = (0..batch.values.len()).map(|_| "?").collect::<Vec<_>>().join(",");
            let query = format!("INSERT OR IGNORE INTO \"{name}\" VALUES({values_placeholder})");
            let mut transaction = connection.begin().await?;
            for row in 0..batch.values[0].len() {
                let mut q = sqlx::query(&query);
                for col in 0..batch.values.len() {
                    q = match &batch.values[col][row] {
                        Value::Int(i) => q.bind(i),
                        Value::Real(f) => q.bind(f),
                        Value::Text(t) => q.bind(t),
                        Value::Blob(b) => q.bind(b),
                        // FIXME: oof, to insert NULL we need to bind None
                        Value::Null => q.bind(Option::<i64>::None)
                    };
                };
                q.execute(&mut *transaction).await?;
            }
            transaction.commit().await?;
            message.ack();
        }
        Ok(())
    }
}


impl<Input, Output> Section<Input, Output, SectionChannel> for Sqlite
    where Input: Stream<Item=Message> + Send + 'static,
          Output: Sink<Message, Error=section::Error> + Send + 'static,
{
    type Error = section::Error;
    type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'static>>;

    fn start(self: Box<Self>, input: Input, output: Output, command: SectionChannel) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, command).await })
    }
}


// FIXME: this conversion doesn't make sense and just wastes cpu cycles
// SqliteRecordBatch needs to be refactored into trait or just use `Message` directly
impl From<&Message> for SqliteRecordBatch{
    fn from(message: &Message) -> Self {
        let arrow_rb = &message.payload.0; 
        let (columns, column_types) = arrow_rb
            .schema()
            .fields
            .into_iter()
            .fold((vec![], vec![]), |(mut columns, mut column_types), field| {
                columns.push(field.name().clone());
                column_types.push(field.data_type().into());
                (columns, column_types)
            });
        let values = arrow_rb.columns().iter().map(|column| {
            let array = column.as_ref();
            // FIXME: is it possible to use downcast_macro from arrow? 
            match array.data_type() {
                DataType::Int8 => {
                    array.as_primitive::<Int8Type>()
                        .into_iter()
                        .map(|x| x.map(|x| x as i64).map(Value::Int).unwrap_or(Value::Null))
                        .collect()
                },
                DataType::Int16 => {
                    array.as_primitive::<Int16Type>()
                        .into_iter()
                        .map(|x| x.map(|x| x as i64).map(Value::Int).unwrap_or(Value::Null))
                        .collect()
                },
                DataType::Int32 => {
                    array.as_primitive::<Int32Type>()
                        .into_iter()
                        .map(|x| x.map(|x| x as i64).map(Value::Int).unwrap_or(Value::Null))
                        .collect()
                },
                DataType::Int64 => {
                    array.as_primitive::<Int64Type>()
                        .into_iter()
                        .map(|x| x.map(Value::Int).unwrap_or(Value::Null))
                        .collect()
                },
                DataType::Float16 => {
                    array.as_primitive::<Float16Type>()
                        .into_iter()
                        .map(|x| x.map(|x| x.into()).map(Value::Real).unwrap_or(Value::Null))
                        .collect()
                },
                DataType::Float32 => {
                    array.as_primitive::<Float32Type>()
                        .into_iter()
                        .map(|x| x.map(|x| x.into()).map(Value::Real).unwrap_or(Value::Null))
                        .collect()
                }, 
                DataType::Float64 => {
                    array.as_primitive::<Float64Type>()
                        .into_iter()
                        .map(|x| x.map(Value::Real).unwrap_or(Value::Null))
                        .collect()
                },
                DataType::Binary => {
                    array.as_binary::<i32>()
                        .into_iter()
                        .map(|x| x.map(|x| Value::Blob(x.into())).unwrap_or(Value::Null))
                        .collect()
                },
                DataType::Utf8 => {
                    array.as_string::<i32>()
                        .into_iter()
                        .map(|x| x.map(|x| Value::Text(x.into())).unwrap_or(Value::Null))
                        .collect()
                },
                dt => unimplemented!("unimplemented {}", dt),
            }
        }).collect();
        SqliteRecordBatch{
            name: Arc::from(message.origin.clone()),
            columns: Arc::from(columns),
            column_types: Arc::from(column_types),
            values,
            // FIXME:
            offset: 0,
        }
    }
}

/// constructor for sqlite destination
///
/// # Config example:
/// ```toml
/// [[section]]
/// name = "sqlite_destination"
/// path = ":memory:"
/// ```
pub fn constructor(config: &Map) -> Result<DynSection, section::Error> {
    let path = config
        .get("path")
        .ok_or("sqlite section requires 'path'")?
        .as_str()
        .ok_or("path should be string")?;
    Ok(Box::new(Sqlite::new(path)))
}
