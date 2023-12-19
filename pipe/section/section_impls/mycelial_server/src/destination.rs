//! Mycelial Net
//!
//! network section, dumps incoming messages to provided http endpoint
use arrow::{
    array::{
        ArrayBuilder, ArrayRef, BinaryArray, BinaryBuilder, BooleanBuilder, Date64Builder,
        Decimal128Builder, Float32Array, Float32Builder, Float64Array, Float64Builder, Int16Array,
        Int16Builder, Int32Array, Int32Builder, Int64Array, Int64Builder, Int8Array, Int8Builder,
        NullBuilder, StringArray, StringBuilder, Time64MicrosecondBuilder,
        TimestampMicrosecondBuilder, UInt16Array, UInt16Builder, UInt32Array, UInt32Builder,
        UInt64Array, UInt64Builder, UInt8Array, UInt8Builder, UnionArray, BooleanArray, Decimal128Array, Time64MicrosecondArray, Date64Array, TimestampMicrosecondArray,
    },
    buffer::Buffer,
    datatypes::{
        DataType as ArrowDataType, Field, Schema, TimeUnit, UnionFields, UnionMode,
        DECIMAL_DEFAULT_SCALE, DECIMAL128_MAX_PRECISION,
    },
    ipc::writer::StreamWriter,
    record_batch::RecordBatch,
};
use base64::engine::{general_purpose::STANDARD as BASE64, Engine};
use reqwest::Body;
use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt, Sink, Stream, StreamExt, TryStreamExt},
    message::{Chunk, Column, DataFrame, DataType, MessageStream, ValueView},
    section::Section,
    SectionError, SectionFuture, SectionMessage, decimal::Decimal,
};
use std::{pin::pin, sync::Arc, collections::HashMap};

#[derive(Debug)]
pub struct Mycelial {
    endpoint: String,
    token: String,
    topic: String,
}

fn into_arrow_datatype(dt: DataType) -> ArrowDataType {
    match dt {
        DataType::I8 => ArrowDataType::Int8,
        DataType::I16 => ArrowDataType::Int16,
        DataType::I32 => ArrowDataType::Int32,
        DataType::I64 => ArrowDataType::Int64,
        DataType::U8 => ArrowDataType::UInt8,
        DataType::U16 => ArrowDataType::UInt16,
        DataType::U32 => ArrowDataType::UInt32,
        DataType::U64 => ArrowDataType::UInt64,
        DataType::F32 => ArrowDataType::Float32,
        DataType::F64 => ArrowDataType::Float64,
        DataType::Str => ArrowDataType::Utf8,
        DataType::Bin => ArrowDataType::Binary,
        DataType::Bool => ArrowDataType::Boolean,
        DataType::Time => ArrowDataType::Time64(TimeUnit::Nanosecond),
        DataType::Date => ArrowDataType::Date64,
        DataType::TimeStamp => ArrowDataType::Timestamp(TimeUnit::Millisecond, None),
        DataType::Null => ArrowDataType::Null,
        _ => unimplemented!("{:?}", dt),
    }
}

// cast rust decimal to i128 with default arrow scale
fn rust_decimal_to_i128(decimal: &Decimal) -> i128 {
    let m = decimal.mantissa();
    let s = decimal.scale();
    match DECIMAL_DEFAULT_SCALE as i32 - s as i32 {
        0 => m,
        v if v < 0 => m / 10_i128.pow(v.unsigned_abs()),
        v => m * 10_i128.pow(v as u32),
    }
}

fn to_union_array(column: Column<'_>) -> Result<(Vec<i8>, Vec<Field>, UnionArray), SectionError> {
    let mut builders: Vec<Option<Box<dyn ArrayBuilder>>> = (0..32).map(|_| None).collect();
    let mut type_ids: Vec<i8> = vec![];
    let mut offsets: Vec<i32> = vec![];
    for value in column {
        let dt = <DataType as Into<i8>>::into(value.data_type());
        type_ids.push(dt);
        let builder = builders
            .get_mut(dt as usize)
            .ok_or(format!("builders array is not big enough to hold {dt}"))?;
        match value {
            ValueView::Null => {
                if builder.is_none() {
                    *builder = Some(Box::new(NullBuilder::new()))
                };
                let b = builder
                    .as_mut()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<NullBuilder>()
                    .unwrap();
                offsets.push(b.len() as i32 as i32);
                b.append_empty_value();
            }
            ValueView::Bool(v) => {
                if builder.is_none() {
                    *builder = Some(Box::new(BooleanBuilder::new()))
                };
                let b = builder
                    .as_mut()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<BooleanBuilder>()
                    .unwrap();
                offsets.push(b.len() as i32);
                b.append_value(v)
            }
            ValueView::I8(v) => {
                if builder.is_none() {
                    *builder = Some(Box::new(Int8Builder::new()))
                };
                let b = builder
                    .as_mut()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<Int8Builder>()
                    .unwrap();
                offsets.push(b.len() as i32);
                b.append_value(v);
            }
            ValueView::I16(v) => {
                if builder.is_none() {
                    *builder = Some(Box::new(Int16Builder::new()))
                };
                let b = builder
                    .as_mut()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<Int16Builder>()
                    .unwrap();
                offsets.push(b.len() as i32);
                b.append_value(v);
            }
            ValueView::I32(v) => {
                if builder.is_none() {
                    *builder = Some(Box::new(Int32Builder::new()))
                };
                let b = builder
                    .as_mut()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<Int32Builder>()
                    .unwrap();
                offsets.push(b.len() as i32);
                b.append_value(v);
            }
            ValueView::I64(v) => {
                if builder.is_none() {
                    *builder = Some(Box::new(Int64Builder::new()))
                };
                let b = builder
                    .as_mut()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<Int64Builder>()
                    .unwrap();
                offsets.push(b.len() as i32);
                b.append_value(v);
            }
            ValueView::U8(v) => {
                if builder.is_none() {
                    *builder = Some(Box::new(UInt8Builder::new()))
                };
                let b = builder
                    .as_mut()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<UInt8Builder>()
                    .unwrap();
                offsets.push(b.len() as i32);
                b.append_value(v);
            }
            ValueView::U16(v) => {
                if builder.is_none() {
                    *builder = Some(Box::new(UInt16Builder::new()))
                };
                let b = builder
                    .as_mut()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<UInt16Builder>()
                    .unwrap();
                offsets.push(b.len() as i32);
                b.append_value(v);
            }
            ValueView::U32(v) => {
                if builder.is_none() {
                    *builder = Some(Box::new(UInt32Builder::new()))
                };
                let b = builder
                    .as_mut()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<UInt32Builder>()
                    .unwrap();
                offsets.push(b.len() as i32);
                b.append_value(v);
            }
            ValueView::U64(v) => {
                if builder.is_none() {
                    *builder = Some(Box::new(UInt64Builder::new()))
                };
                let b = builder
                    .as_mut()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<UInt64Builder>()
                    .unwrap();
                offsets.push(b.len() as i32);
                b.append_value(v);
            }
            ValueView::F32(v) => {
                if builder.is_none() {
                    *builder = Some(Box::new(Float32Builder::new()))
                };
                let b = builder
                    .as_mut()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<Float32Builder>()
                    .unwrap();
                offsets.push(b.len() as i32);
                b.append_value(v);
            }
            ValueView::F64(v) => {
                if builder.is_none() {
                    *builder = Some(Box::new(Float64Builder::new()))
                };
                let b = builder
                    .as_mut()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<Float64Builder>()
                    .unwrap();
                offsets.push(b.len() as i32);
                b.append_value(v);
            }
            ValueView::Str(v) => {
                if builder.is_none() {
                    *builder = Some(Box::new(StringBuilder::new()))
                };
                let b = builder
                    .as_mut()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<StringBuilder>()
                    .unwrap();
                offsets.push(b.len() as i32);
                b.append_value(v);
            }
            ValueView::Bin(v) => {
                if builder.is_none() {
                    *builder = Some(Box::new(BinaryBuilder::new()))
                };
                let b = builder
                    .as_mut()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<BinaryBuilder>()
                    .unwrap();
                offsets.push(b.len() as i32);
                b.append_value(v);
            }
            ValueView::Time(v) => {
                if builder.is_none() {
                    *builder = Some(Box::new(Time64MicrosecondBuilder::new()))
                };
                let b = builder
                    .as_mut()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<Time64MicrosecondBuilder>()
                    .unwrap();
                offsets.push(b.len() as i32);
                b.append_value(v);
            }
            ValueView::Date(v) => {
                if builder.is_none() {
                    *builder = Some(Box::new(Date64Builder::new()))
                };
                builder
                    .as_mut()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<Date64Builder>()
                    .unwrap()
                    .append_value(v / 1000); // micros to millis
            }
            ValueView::TimeStamp(v) => {
                if builder.is_none() {
                    *builder = Some(Box::new(TimestampMicrosecondBuilder::new()))
                };
                builder
                    .as_mut()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<TimestampMicrosecondBuilder>()
                    .unwrap()
                    .append_value(v);
            }
            ValueView::Decimal(v) => {
                if builder.is_none() {
                    *builder = Some(Box::new(Decimal128Builder::new()))
                };
                builder
                    .as_mut()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<Decimal128Builder>()
                    .unwrap()
                    .append_value(rust_decimal_to_i128(v));
            }
            ValueView::Uuid(v) => {
                if builder.is_none() {
                    *builder = Some(Box::new(StringBuilder::new()))
                };
                builder
                    .as_mut()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<StringBuilder>()
                    .unwrap()
                    .append_value(v.to_string());
            }
            _ => Err(format!("unsupported data type: {:?}", dt))?,
        }
    }
    let (field_type_ids, arrays, fields) = builders.into_iter().enumerate().fold(
        (vec![], vec![], vec![]),
        |(mut field_type_ids, mut arrays, mut fields), (type_id, builder)| {
            if builder.is_none() {
                return (field_type_ids, arrays, fields);
            };
            let mut builder = builder.unwrap();
            field_type_ids.push(type_id as i8);
            let dt = DataType::from(type_id as i8);
            let field = Field::new(dt.to_string(), into_arrow_datatype(dt), true)
                .with_metadata(HashMap::from([("mycelial_type".into(), dt.to_string())]));
            fields.push(field.clone());
            arrays.push((field, builder.finish()));
            (field_type_ids, arrays, fields)
        },
    );
    let type_ids_buffer = Buffer::from_slice_ref(&type_ids);
    let offsets = Buffer::from_vec(offsets);
    let union_array = UnionArray::try_new(
        field_type_ids.as_slice(),
        type_ids_buffer,
        Some(offsets),
        arrays,
    )?;
    Ok((field_type_ids, fields, union_array))
}

fn df_to_recordbatch(df: Box<dyn DataFrame>) -> Result<RecordBatch, SectionError> {
    let columns = df.columns();
    let mut schema_columns = Vec::<Field>::with_capacity(columns.len());
    let mut rb_columns = Vec::<ArrayRef>::with_capacity(columns.len());
    for column in df.columns() {
        let name = column.name();
        let dt = column.data_type();
        let (field, arr): (Field, ArrayRef) = match dt {
            DataType::I8 => (
                Field::new(name, ArrowDataType::Int8, true),
                Arc::new(Int8Array::from_iter(column.map(|val| match val {
                    ValueView::I8(i) => Some(i),
                    ValueView::Null => None,
                    _ => panic!("expected i8, got: {:?}", val),
                }))),
            ),
            DataType::I16 => (
                Field::new(name, ArrowDataType::Int16, true),
                Arc::new(Int16Array::from_iter(column.map(|val| match val {
                    ValueView::I16(i) => Some(i),
                    ValueView::Null => None,
                    _ => panic!("expected i16, got: {:?}", val),
                }))),
            ),
            DataType::I32 => (
                Field::new(name, ArrowDataType::Int32, true),
                Arc::new(Int32Array::from_iter(column.map(|val| match val {
                    ValueView::I32(i) => Some(i),
                    ValueView::Null => None,
                    _ => panic!("expected i32, got: {:?}", val),
                }))),
            ),
            DataType::I64 => (
                Field::new(name, ArrowDataType::Int64, true),
                Arc::new(Int64Array::from_iter(column.map(|val| match val {
                    ValueView::I64(i) => Some(i),
                    ValueView::Null => None,
                    _ => panic!("expected i64, got: {:?}", val),
                }))),
            ),
            DataType::U8 => (
                Field::new(name, ArrowDataType::UInt8, true),
                Arc::new(UInt8Array::from_iter(column.map(|val| match val {
                    ValueView::U8(u) => Some(u),
                    ValueView::Null => None,
                    _ => panic!("expected u8, got: {:?}", val),
                }))),
            ),
            DataType::U16 => (
                Field::new(name, ArrowDataType::UInt16, true),
                Arc::new(UInt16Array::from_iter(column.map(|val| match val {
                    ValueView::U16(u) => Some(u),
                    ValueView::Null => None,
                    _ => panic!("expected u16, got: {:?}", val),
                }))),
            ),
            DataType::U32 => (
                Field::new(name, ArrowDataType::UInt32, true),
                Arc::new(UInt32Array::from_iter(column.map(|val| match val {
                    ValueView::U32(u) => Some(u),
                    ValueView::Null => None,
                    _ => panic!("expected u32, got: {:?}", val),
                }))),
            ),
            DataType::U64 => (
                Field::new(name, ArrowDataType::UInt64, true),
                Arc::new(UInt64Array::from_iter(column.map(|val| match val {
                    ValueView::U64(u) => Some(u),
                    ValueView::Null => None,
                    _ => panic!("expected u64, got: {:?}", val),
                }))),
            ),
            DataType::F32 => (
                Field::new(name, ArrowDataType::Float32, true),
                Arc::new(Float32Array::from_iter(column.map(|val| match val {
                    ValueView::F32(u) => Some(u),
                    ValueView::Null => None,
                    _ => panic!("expected f32, got: {:?}", val),
                }))),
            ),
            DataType::F64 => (
                Field::new(name, ArrowDataType::Float64, true),
                Arc::new(Float64Array::from_iter(column.map(|val| match val {
                    ValueView::F64(u) => Some(u),
                    ValueView::Null => None,
                    _ => panic!("expected f64, got: {:?}", val),
                }))),
            ),
            DataType::Str => (
                Field::new(name, ArrowDataType::Utf8, true),
                Arc::new(StringArray::from_iter(column.map(|val| match val {
                    ValueView::Str(s) => Some(s),
                    ValueView::Null => None,
                    _ => panic!("expected Str, got: {:?}", val),
                }))),
            ),
            DataType::Bin => (
                Field::new(name, ArrowDataType::Binary, true),
                Arc::new(BinaryArray::from_iter(column.map(|val| match val {
                    ValueView::Bin(s) => Some(s),
                    ValueView::Null => None,
                    _ => panic!("expected u64, got: {:?}", val),
                }))),
            ),
            DataType::RawJson => (
                Field::new(name, ArrowDataType::Utf8, true),
                Arc::new(StringArray::from_iter(column.map(|val| match val {
                    ValueView::Str(s) => Some(s),
                    ValueView::Null => None,
                    _ => panic!("expected string, got: {:?}", val),
                })))
            ),
            DataType::Bool => (
                Field::new(name, ArrowDataType::Boolean, true),
                Arc::new(BooleanArray::from_iter(column.map(|val| match val {
                    ValueView::Bool(b) => Some(b),
                    ValueView::Null => None,
                    _ => panic!("expected bool, got: {:?}", val),
                })))
            ),
            DataType::Decimal => (
                Field::new(name, ArrowDataType::Decimal128(DECIMAL128_MAX_PRECISION, DECIMAL_DEFAULT_SCALE), true),
                Arc::new(Decimal128Array::from_iter(column.map(|val| match val {
                    ValueView::Decimal(d) => Some(rust_decimal_to_i128(d)),
                    ValueView::Null => None,
                    _ => panic!("expected decimal, got: {:?}", val),
                })))
            ),
            DataType::Uuid => (
                Field::new(name, ArrowDataType::Utf8, true),
                Arc::new(StringArray::from_iter(column.map(|val| match val {
                    ValueView::Uuid(u) => Some(u.to_string()),
                    ValueView::Null => None,
                    _ => panic!("expected uuid, got: {:?}", val),
                })))
            ),
            DataType::Time => (
                Field::new(name, ArrowDataType::Time64(TimeUnit::Nanosecond), true),
                Arc::new(Time64MicrosecondArray::from_iter(column.map(|val| match val {
                    ValueView::Time(v) => Some(v),
                    ValueView::Null => None,
                    _ => panic!("expected time, got: {:?}", val),
                })))
            ),
            DataType::Date => (
                Field::new(name, ArrowDataType::Date64, true),
                Arc::new(Date64Array::from_iter(column.map(|val| match val {
                    ValueView::Date(d) => Some(d / 1000), // micros to millis
                    ValueView::Null => None,
                    _ => panic!("expected date, got: {:?}", val),
                })))
            ),
            DataType::TimeStamp => (
                Field::new(name, ArrowDataType::Timestamp(TimeUnit::Nanosecond, None), true),
                Arc::new(TimestampMicrosecondArray::from_iter(column.map(|val| match val {
                    ValueView::TimeStamp(v) => Some(v),
                    ValueView::Null => None,
                    _ => panic!("expected timestamp, got: {:?}", val),
                })))
            ),
            DataType::Any => {
                let name = name.to_string();
                let (type_ids, fields, union_array) = to_union_array(column)?;
                let dt = ArrowDataType::Union(UnionFields::new(type_ids, fields), UnionMode::Dense);
                (Field::new(name, dt, true), Arc::new(union_array))
            }
            dt => unimplemented!("unimplemented dt: {:?}", dt),
        };
        let field = field.with_metadata(HashMap::from([("mycelial_type".into(), dt.to_string())]));
        schema_columns.push(field);
        rb_columns.push(arr);
    }

    Ok(RecordBatch::try_new(
        Arc::new(Schema::new(schema_columns)),
        rb_columns,
    )?)
}

impl Mycelial {
    pub fn new(
        endpoint: impl Into<String>,
        token: impl Into<String>,
        topic: impl Into<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            token: token.into(),
            topic: topic.into(),
        }
    }

    pub async fn enter_loop<Input, Output, SectionChan>(
        self,
        input: Input,
        _output: Output,
        mut section_chan: SectionChan,
    ) -> Result<(), SectionError>
    where
        Input: Stream<Item = SectionMessage> + Send,
        Output: Sink<SectionMessage, Error = SectionError> + Send,
        SectionChan: SectionChannel,
    {
        let mut input = pin!(input.fuse());
        let client = &mut reqwest::Client::new();
        loop {
            futures::select! {
                cmd = section_chan.recv().fuse() => {
                    if let Command::Stop = cmd? {
                        return Ok(())
                    }
                },

                msg = input.next() => {
                    let mut msg = match msg {
                        Some(msg) => msg,
                        None => Err("input stream closed")?
                    };
                    let origin = msg.origin().to_string();
                    let ack = msg.ack();
                    let msg_stream: MessageStream = msg.into();
                    let msg_stream = msg_stream
                        .map_ok(|chunk| {
                            match chunk {
                                Chunk::DataFrame(df) => {
                                    // FIXME: unwrap unwrap unwrap
                                    let rb = df_to_recordbatch(df).unwrap();
                                    let mut stream_writer: StreamWriter<_> = StreamWriter::try_new(vec![], rb.schema().as_ref()).unwrap();
                                    stream_writer.write(&rb).unwrap();
                                    stream_writer.finish().unwrap();

                                    stream_writer.into_inner().unwrap()
                                },
                                Chunk::Byte(bin) => bin,
                            }
                        });
                    let body = Body::wrap_stream(msg_stream);
                    let _ = client
                        .post(format!(
                            "{}/{}",
                            self.endpoint.as_str().trim_end_matches('/'),
                            self.topic
                        ))
                        .header("Authorization", self.basic_auth())
                        .header("x-message-origin", origin)
                        .body(body)
                        .send()
                        .await?;
                    ack.await;
                },
            }
        }
    }

    fn basic_auth(&self) -> String {
        format!("Basic {}", BASE64.encode(format!("{}:", self.token)))
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Mycelial
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel,
{
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(self, input: Input, output: Output, section_chan: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, section_chan).await })
    }
}
