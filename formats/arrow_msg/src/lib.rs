pub use arrow;

use std::{collections::HashMap, sync::Arc};

use arrow::{
    array::{
        as_union_array, Array, ArrayBuilder, ArrayRef, AsArray, BinaryArray, BinaryBuilder,
        BooleanArray, BooleanBuilder, Date64Array, Date64Builder, Decimal128Array,
        Decimal128Builder, Float32Array, Float32Builder, Float64Array, Float64Builder, Int16Array,
        Int16Builder, Int32Array, Int32Builder, Int64Array, Int64Builder, Int8Array, Int8Builder,
        NullBuilder, StringArray, StringBuilder, Time64MicrosecondArray, Time64MicrosecondBuilder,
        Time64NanosecondArray, TimestampMicrosecondArray, TimestampMicrosecondBuilder,
        TimestampMillisecondArray, TimestampNanosecondArray, TimestampSecondArray, UInt16Array,
        UInt16Builder, UInt32Array, UInt32Builder, UInt64Array, UInt64Builder, UInt8Array,
        UInt8Builder, UnionArray,
    },
    buffer::Buffer,
    datatypes::{
        DataType as ArrowDataType, Date32Type, Date64Type, Decimal128Type, Field, Float32Type,
        Float64Type, Int16Type, Int32Type, Int64Type, Int8Type, Schema, SchemaRef,
        Time32MillisecondType, Time32SecondType, Time64MicrosecondType, Time64NanosecondType,
        TimeUnit as ArrowTimeUnit, TimestampMicrosecondType, TimestampMillisecondType,
        TimestampNanosecondType, TimestampSecondType, UInt16Type, UInt32Type, UInt64Type,
        UInt8Type, UnionFields, UnionMode, DECIMAL128_MAX_PRECISION, DECIMAL_DEFAULT_SCALE,
    },
    record_batch::RecordBatch as ArrowRecordBatch,
};
use chrono::FixedOffset;
use section::{
    decimal::Decimal,
    message::{Ack, Chunk, Column, DataFrame, DataType, Message, TimeUnit, ValueView},
    SectionError,
};

// Wrap around arrow record batch, which implements dataframe
#[derive(Debug)]
pub struct RecordBatch {
    inner: ArrowRecordBatch,
    schema: SchemaRef,
}

impl RecordBatch {
    pub fn new(inner: ArrowRecordBatch) -> Self {
        let schema = inner.schema();
        Self { inner, schema }
    }
}

impl From<ArrowRecordBatch> for RecordBatch {
    fn from(value: ArrowRecordBatch) -> Self {
        Self::new(value)
    }
}

pub struct ArrowMsg {
    inner: Vec<Option<RecordBatch>>,
    pos: usize,
    ack: Option<Ack>,
    origin: String,
}

impl std::fmt::Debug for ArrowMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArrowMsg")
            .field("inner", &self.inner)
            .field("pos", &self.pos)
            .field("origin", &self.origin)
            .finish()
    }
}

impl ArrowMsg {
    pub fn new(
        origin: impl Into<String>,
        inner: Vec<Option<RecordBatch>>,
        ack: Option<Ack>,
    ) -> Self {
        Self {
            origin: origin.into(),
            inner,
            ack,
            pos: 0,
        }
    }
}

impl Message for ArrowMsg {
    fn origin(&self) -> &str {
        self.origin.as_str()
    }

    fn next(&mut self) -> section::message::Next<'_> {
        match self.pos >= self.inner.len() {
            true => Box::pin(async { Ok(None) }),
            false => {
                let rb = self.inner[self.pos].take().unwrap();
                self.pos += 1;
                Box::pin(async move { Ok(Some(Chunk::DataFrame(Box::new(rb)))) })
            }
        }
    }

    fn ack(&mut self) -> section::message::Ack {
        self.ack.take().unwrap_or(Box::pin(async {}))
    }
}

fn union_array_to_iter<'a>(
    union_fields: &'a UnionFields,
    array: &'a UnionArray,
) -> Box<dyn Iterator<Item = ValueView<'a>> + Send + 'a> {
    let field_map: HashMap<i8, &Arc<Field>> = union_fields.iter().collect();
    let iter = (0..array.len()).map(move |index| {
        let type_id = array.type_id(index);
        let value_offset = array.value_offset(index);
        let child = array.child(type_id);
        let field = field_map.get(&type_id).unwrap();
        match field.data_type() {
            ArrowDataType::Null => ValueView::Null,
            ArrowDataType::Int8 => {
                ValueView::I8(child.as_primitive::<Int8Type>().value(value_offset))
            }
            ArrowDataType::Int16 => {
                ValueView::I16(child.as_primitive::<Int16Type>().value(value_offset))
            }
            ArrowDataType::Int32 => {
                ValueView::I32(child.as_primitive::<Int32Type>().value(value_offset))
            }
            ArrowDataType::Int64 => {
                ValueView::I64(child.as_primitive::<Int64Type>().value(value_offset))
            }
            ArrowDataType::UInt8 => {
                ValueView::U8(child.as_primitive::<UInt8Type>().value(value_offset))
            }
            ArrowDataType::UInt16 => {
                ValueView::U16(child.as_primitive::<UInt16Type>().value(value_offset))
            }
            ArrowDataType::UInt32 => {
                ValueView::U32(child.as_primitive::<UInt32Type>().value(value_offset))
            }
            ArrowDataType::UInt64 => {
                ValueView::U64(child.as_primitive::<UInt64Type>().value(value_offset))
            }
            ArrowDataType::Float32 => {
                ValueView::F32(child.as_primitive::<Float32Type>().value(value_offset))
            }
            ArrowDataType::Float64 => {
                ValueView::F64(child.as_primitive::<Float64Type>().value(value_offset))
            }
            ArrowDataType::Utf8 => ValueView::Str(child.as_string::<i32>().value(value_offset)),
            ArrowDataType::LargeUtf8 => {
                ValueView::Str(child.as_string::<i32>().value(value_offset))
            }
            ArrowDataType::Binary => ValueView::Bin(child.as_binary::<i32>().value(value_offset)),
            ArrowDataType::LargeBinary => {
                ValueView::Bin(child.as_binary::<i32>().value(value_offset))
            }
            ArrowDataType::Decimal128(scale, _precision) => ValueView::Decimal({
                let d = child.as_primitive::<Decimal128Type>().value(value_offset);
                Decimal::from_i128_with_scale(d, *scale as _)
            }),
            ArrowDataType::Time32(tu) => match tu {
                ArrowTimeUnit::Second => ValueView::Time(
                    TimeUnit::Second,
                    child.as_primitive::<Time32SecondType>().value(value_offset) as _,
                ),
                ArrowTimeUnit::Millisecond => ValueView::Time(
                    TimeUnit::Millisecond,
                    child
                        .as_primitive::<Time32MillisecondType>()
                        .value(value_offset) as _,
                ),
                _ => unreachable!("time32 encodes only seconds or milliseconds"),
            },
            ArrowDataType::Time64(tu) => match tu {
                ArrowTimeUnit::Microsecond => ValueView::Time(
                    TimeUnit::Microsecond,
                    child
                        .as_primitive::<Time64MicrosecondType>()
                        .value(value_offset),
                ),
                ArrowTimeUnit::Nanosecond => ValueView::Time(
                    TimeUnit::Nanosecond,
                    child
                        .as_primitive::<Time64NanosecondType>()
                        .value(value_offset),
                ),
                _ => unreachable!("time64 encodes only microseconds or nanoseconds"),
            },
            ArrowDataType::Date32 => ValueView::Date(
                TimeUnit::Second,
                child.as_primitive::<Date32Type>().value(value_offset) as i64 * 86400,
            ),
            ArrowDataType::Date64 => ValueView::Date(
                TimeUnit::Millisecond,
                child.as_primitive::<Date64Type>().value(value_offset),
            ),
            ArrowDataType::Timestamp(tu, tz) => {
                let value = match tu {
                    ArrowTimeUnit::Second => child
                        .as_primitive::<TimestampSecondType>()
                        .value(value_offset),
                    ArrowTimeUnit::Millisecond => child
                        .as_primitive::<TimestampMillisecondType>()
                        .value(value_offset),
                    ArrowTimeUnit::Microsecond => child
                        .as_primitive::<TimestampMicrosecondType>()
                        .value(value_offset),
                    ArrowTimeUnit::Nanosecond => child
                        .as_primitive::<TimestampNanosecondType>()
                        .value(value_offset),
                };
                let tu = from_arrow_timeunit(tu);
                match tz {
                    None => ValueView::TimeStamp(tu, value),
                    Some(tz) => {
                        let offset: i64 = tz
                            .as_ref()
                            .parse::<FixedOffset>()
                            .unwrap()
                            .utc_minus_local() as i64;
                        ValueView::TimeStampUTC(tu, value + offset)
                    }
                }
            }
            dt => unimplemented!("unimplemented dt: {}", dt),
        }
    });
    Box::new(iter)
}

impl DataFrame for RecordBatch {
    fn columns(&self) -> Vec<section::message::Column<'_>> {
        self.schema
            .fields()
            .iter()
            .zip(self.inner.columns())
            .map(|(field, column)| {
                let (dt, iter): (DataType, Box<dyn Iterator<Item = ValueView> + Send>) = match field
                    .data_type()
                {
                    ArrowDataType::Int8 => {
                        let arr = column.as_primitive::<Int8Type>();
                        (
                            DataType::I8,
                            Box::new(
                                arr.iter()
                                    .map(|val| val.map(ValueView::I8).unwrap_or(ValueView::Null)),
                            ),
                        )
                    }
                    ArrowDataType::Int16 => {
                        let arr = column.as_primitive::<Int16Type>();
                        (
                            DataType::I16,
                            Box::new(
                                arr.iter()
                                    .map(|val| val.map(ValueView::I16).unwrap_or(ValueView::Null)),
                            ),
                        )
                    }
                    ArrowDataType::Int32 => {
                        let arr = column.as_primitive::<Int32Type>();
                        (
                            DataType::I32,
                            Box::new(
                                arr.iter()
                                    .map(|val| val.map(ValueView::I32).unwrap_or(ValueView::Null)),
                            ),
                        )
                    }
                    ArrowDataType::Int64 => {
                        let arr = column.as_primitive::<Int64Type>();
                        (
                            DataType::I64,
                            Box::new(
                                arr.iter()
                                    .map(|val| val.map(ValueView::I64).unwrap_or(ValueView::Null)),
                            ),
                        )
                    }
                    ArrowDataType::UInt8 => {
                        let arr = column.as_primitive::<UInt8Type>();
                        (
                            DataType::U8,
                            Box::new(
                                arr.iter()
                                    .map(|val| val.map(ValueView::U8).unwrap_or(ValueView::Null)),
                            ),
                        )
                    }
                    ArrowDataType::UInt16 => {
                        let arr = column.as_primitive::<UInt16Type>();
                        (
                            DataType::U16,
                            Box::new(
                                arr.iter()
                                    .map(|val| val.map(ValueView::U16).unwrap_or(ValueView::Null)),
                            ),
                        )
                    }
                    ArrowDataType::UInt32 => {
                        let arr = column.as_primitive::<UInt32Type>();
                        (
                            DataType::U32,
                            Box::new(
                                arr.iter()
                                    .map(|val| val.map(ValueView::U32).unwrap_or(ValueView::Null)),
                            ),
                        )
                    }
                    ArrowDataType::UInt64 => {
                        let arr = column.as_primitive::<UInt64Type>();
                        (
                            DataType::U64,
                            Box::new(
                                arr.iter()
                                    .map(|val| val.map(ValueView::U64).unwrap_or(ValueView::Null)),
                            ),
                        )
                    }
                    ArrowDataType::Float32 => {
                        let arr = column.as_primitive::<Float32Type>();
                        (
                            DataType::F32,
                            Box::new(
                                arr.iter()
                                    .map(|val| val.map(ValueView::F32).unwrap_or(ValueView::Null)),
                            ),
                        )
                    }
                    ArrowDataType::Float64 => {
                        let arr = column.as_primitive::<Float64Type>();
                        (
                            DataType::F64,
                            Box::new(
                                arr.iter()
                                    .map(|val| val.map(ValueView::F64).unwrap_or(ValueView::Null)),
                            ),
                        )
                    }
                    ArrowDataType::Utf8 => {
                        let arr = column.as_string::<i32>();
                        (
                            DataType::Str,
                            Box::new(
                                arr.iter()
                                    .map(|val| val.map(ValueView::Str).unwrap_or(ValueView::Null)),
                            ),
                        )
                    }
                    ArrowDataType::Binary => {
                        let arr = column.as_binary::<i32>();
                        (
                            DataType::Bin,
                            Box::new(
                                arr.iter()
                                    .map(|val| val.map(ValueView::Bin).unwrap_or(ValueView::Null)),
                            ),
                        )
                    }
                    ArrowDataType::Boolean => {
                        let arr = column.as_boolean();
                        (
                            DataType::Bool,
                            Box::new(
                                arr.iter()
                                    .map(|val| val.map(ValueView::Bool).unwrap_or(ValueView::Null)),
                            ),
                        )
                    }
                    ArrowDataType::Time32(time_unit) => match time_unit {
                        ArrowTimeUnit::Second => {
                            let arr = column.as_primitive::<Time32SecondType>();
                            (
                                DataType::Time(TimeUnit::Second),
                                Box::new(arr.iter().map(|val| {
                                    val.map(|v| ValueView::Time(TimeUnit::Second, v as _))
                                        .unwrap_or(ValueView::Null)
                                })),
                            )
                        }
                        ArrowTimeUnit::Millisecond => {
                            let arr = column.as_primitive::<Time32MillisecondType>();
                            (
                                DataType::Time(TimeUnit::Millisecond),
                                Box::new(arr.iter().map(|val| {
                                    val.map(|v| ValueView::Time(TimeUnit::Millisecond, v as _))
                                        .unwrap_or(ValueView::Null)
                                })),
                            )
                        }
                        _ => unimplemented!("Time32 only supports time unit in seconds"),
                    },
                    ArrowDataType::Time64(tu) => {
                        let tu = from_arrow_timeunit(tu);
                        let iter: Box<dyn Iterator<Item = ValueView> + Send> = match tu {
                            TimeUnit::Microsecond => {
                                let arr = column.as_primitive::<Time64MicrosecondType>();
                                Box::new(arr.iter().map(move |val| {
                                    val.map(|v| ValueView::Time(tu, v))
                                        .unwrap_or(ValueView::Null)
                                }))
                            }
                            TimeUnit::Nanosecond => {
                                let arr = column.as_primitive::<Time64NanosecondType>();
                                Box::new(arr.iter().map(move |val| {
                                    val.map(|v| ValueView::Time(tu, v))
                                        .unwrap_or(ValueView::Null)
                                }))
                            }
                            _ => unreachable!(
                                "arrow time64 can carry only microseconds and nanoseconds"
                            ),
                        };
                        (DataType::Time(tu), iter)
                    }
                    ArrowDataType::Date32 => {
                        let arr = column.as_primitive::<Date32Type>();
                        (
                            DataType::Date(TimeUnit::Second),
                            Box::new(arr.iter().map(|val| {
                                val.map(|v| ValueView::Date(TimeUnit::Second, (v as i64) * 86400))
                                    .unwrap_or(ValueView::Null)
                            })),
                        )
                    }
                    ArrowDataType::Date64 => {
                        let arr = column.as_primitive::<Date64Type>();
                        (
                            DataType::Date(TimeUnit::Millisecond),
                            Box::new(arr.iter().map(|val| {
                                val.map(|v| ValueView::Date(TimeUnit::Millisecond, v))
                                    .unwrap_or(ValueView::Null)
                            })),
                        )
                    }
                    ArrowDataType::Timestamp(tu, None) => match tu {
                        ArrowTimeUnit::Second => {
                            let arr = column.as_primitive::<TimestampSecondType>();
                            (
                                DataType::TimeStamp(TimeUnit::Second),
                                Box::new(arr.iter().map(|val| {
                                    val.map(|v| ValueView::TimeStamp(TimeUnit::Second, v))
                                        .unwrap_or(ValueView::Null)
                                })),
                            )
                        }
                        ArrowTimeUnit::Millisecond => {
                            let arr = column.as_primitive::<TimestampMillisecondType>();
                            (
                                DataType::TimeStamp(TimeUnit::Millisecond),
                                Box::new(arr.iter().map(|val| {
                                    val.map(|v| ValueView::TimeStamp(TimeUnit::Millisecond, v))
                                        .unwrap_or(ValueView::Null)
                                })),
                            )
                        }
                        ArrowTimeUnit::Microsecond => {
                            let arr = column.as_primitive::<TimestampMicrosecondType>();
                            (
                                DataType::TimeStamp(TimeUnit::Microsecond),
                                Box::new(arr.iter().map(|val| {
                                    val.map(|v| ValueView::TimeStamp(TimeUnit::Microsecond, v))
                                        .unwrap_or(ValueView::Null)
                                })),
                            )
                        }
                        ArrowTimeUnit::Nanosecond => {
                            let arr = column.as_primitive::<TimestampNanosecondType>();
                            (
                                DataType::TimeStamp(TimeUnit::Nanosecond),
                                Box::new(arr.iter().map(|val| {
                                    val.map(|v| ValueView::TimeStamp(TimeUnit::Nanosecond, v))
                                        .unwrap_or(ValueView::Null)
                                })),
                            )
                        }
                    },
                    ArrowDataType::Timestamp(tu, Some(tz)) => {
                        // FIXME: unwrap
                        let offset: i64 = tz
                            .as_ref()
                            .parse::<FixedOffset>()
                            .unwrap()
                            .utc_minus_local() as i64;
                        match tu {
                            ArrowTimeUnit::Second => {
                                let arr = column.as_primitive::<TimestampSecondType>();
                                (
                                    DataType::TimeStampUTC(TimeUnit::Second),
                                    Box::new(arr.iter().map(move |val| {
                                        val.map(|v| {
                                            ValueView::TimeStampUTC(TimeUnit::Second, v + offset)
                                        })
                                        .unwrap_or(ValueView::Null)
                                    })),
                                )
                            }
                            ArrowTimeUnit::Millisecond => {
                                let arr = column.as_primitive::<TimestampMillisecondType>();
                                (
                                    DataType::TimeStampUTC(TimeUnit::Millisecond),
                                    Box::new(arr.iter().map(move |val| {
                                        val.map(|v| {
                                            ValueView::TimeStampUTC(
                                                TimeUnit::Millisecond,
                                                v + offset * 1000,
                                            )
                                        })
                                        .unwrap_or(ValueView::Null)
                                    })),
                                )
                            }
                            ArrowTimeUnit::Microsecond => {
                                let arr = column.as_primitive::<TimestampMicrosecondType>();
                                (
                                    DataType::TimeStampUTC(TimeUnit::Microsecond),
                                    Box::new(arr.iter().map(move |val| {
                                        val.map(|v| {
                                            ValueView::TimeStampUTC(
                                                TimeUnit::Microsecond,
                                                v + offset * 1_000_000,
                                            )
                                        })
                                        .unwrap_or(ValueView::Null)
                                    })),
                                )
                            }
                            ArrowTimeUnit::Nanosecond => {
                                let arr = column.as_primitive::<TimestampNanosecondType>();
                                (
                                    DataType::TimeStampUTC(TimeUnit::Nanosecond),
                                    Box::new(arr.iter().map(move |val| {
                                        val.map(|v| {
                                            ValueView::TimeStampUTC(
                                                TimeUnit::Nanosecond,
                                                v + offset * 1_000_000_000,
                                            )
                                        })
                                        .unwrap_or(ValueView::Null)
                                    })),
                                )
                            }
                        }
                    }
                    ArrowDataType::Null => {
                        let arr = column.as_primitive::<Int8Type>();
                        (
                            DataType::Null,
                            Box::new(arr.iter().map(|_| ValueView::Null)),
                        )
                    }
                    ArrowDataType::Decimal128(_precision, scale) => {
                        let arr = column.as_primitive::<Decimal128Type>();
                        (
                            DataType::Decimal,
                            Box::new(arr.iter().map(|val| {
                                val.map(|num| {
                                    ValueView::Decimal(Decimal::from_i128_with_scale(
                                        num,
                                        *scale as _,
                                    ))
                                })
                                .unwrap_or(ValueView::Null)
                            })),
                        )
                    }
                    ArrowDataType::Union(uf, _mode) => (
                        DataType::Any,
                        union_array_to_iter(uf, as_union_array(column)),
                    ),
                    dt => panic!("unsupported arrow datatype: {:?}", dt),
                };
                Column::new(field.name(), dt, iter)
            })
            .collect()
    }
}

fn into_arrow_timeunit(tu: TimeUnit) -> ArrowTimeUnit {
    match tu {
        TimeUnit::Second => ArrowTimeUnit::Second,
        TimeUnit::Millisecond => ArrowTimeUnit::Millisecond,
        TimeUnit::Microsecond => ArrowTimeUnit::Microsecond,
        TimeUnit::Nanosecond => ArrowTimeUnit::Nanosecond,
    }
}

fn from_arrow_timeunit(tu: &ArrowTimeUnit) -> TimeUnit {
    match tu {
        ArrowTimeUnit::Second => TimeUnit::Second,
        ArrowTimeUnit::Millisecond => TimeUnit::Millisecond,
        ArrowTimeUnit::Microsecond => TimeUnit::Microsecond,
        ArrowTimeUnit::Nanosecond => TimeUnit::Nanosecond,
    }
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
        DataType::Time(tu) if tu == TimeUnit::Second || tu == TimeUnit::Millisecond => {
            ArrowDataType::Time32(into_arrow_timeunit(tu))
        }
        DataType::Time(tu) => ArrowDataType::Time64(into_arrow_timeunit(tu)),
        DataType::Date(_) => ArrowDataType::Date64,
        DataType::TimeStamp(tu) => ArrowDataType::Timestamp(into_arrow_timeunit(tu), None),
        DataType::TimeStampUTC(tu) => {
            ArrowDataType::Timestamp(into_arrow_timeunit(tu), Some(Arc::from("UTC")))
        }
        DataType::Null => ArrowDataType::Null,
        _ => unimplemented!("{:?}", dt),
    }
}

// cast rust decimal to i128 with default arrow scale
fn rust_decimal_to_i128(decimal: Decimal) -> i128 {
    let m = decimal.mantissa();
    let s = decimal.scale();
    match DECIMAL_DEFAULT_SCALE as i32 - s as i32 {
        0 => m,
        v if v < 0 => m / 10_i128.pow(v.unsigned_abs()),
        v => m * 10_i128.pow(v as u32),
    }
}

fn to_union_array(column: Column<'_>) -> Result<(Vec<i8>, Vec<Field>, UnionArray), SectionError> {
    let mut builders: Vec<Option<Box<dyn ArrayBuilder>>> = (0..=32).map(|_| None).collect();
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
                offsets.push(b.len() as _);
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
            ValueView::Time(_tu, v) => {
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
            ValueView::Date(tu, v) => {
                if builder.is_none() {
                    *builder = Some(Box::new(Date64Builder::new()))
                };

                let v = match tu {
                    TimeUnit::Second => v * 1000,
                    TimeUnit::Millisecond => v,
                    TimeUnit::Microsecond => v / 1000,
                    TimeUnit::Nanosecond => v / 1_000_000,
                };
                builder
                    .as_mut()
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<Date64Builder>()
                    .unwrap()
                    .append_value(v);
            }
            ValueView::TimeStamp(_tu, v) => {
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

pub fn df_to_recordbatch(df: &dyn DataFrame) -> Result<ArrowRecordBatch, SectionError> {
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
                }))),
            ),
            DataType::Bool => (
                Field::new(name, ArrowDataType::Boolean, true),
                Arc::new(BooleanArray::from_iter(column.map(|val| match val {
                    ValueView::Bool(b) => Some(b),
                    ValueView::Null => None,
                    _ => panic!("expected bool, got: {:?}", val),
                }))),
            ),
            DataType::Decimal => (
                Field::new(
                    name,
                    ArrowDataType::Decimal128(DECIMAL128_MAX_PRECISION, DECIMAL_DEFAULT_SCALE),
                    true,
                ),
                Arc::new(Decimal128Array::from_iter(column.map(|val| match val {
                    ValueView::Decimal(d) => Some(rust_decimal_to_i128(d)),
                    ValueView::Null => None,
                    _ => panic!("expected decimal, got: {:?}", val),
                }))),
            ),
            DataType::Uuid => (
                Field::new(name, ArrowDataType::Utf8, true),
                Arc::new(StringArray::from_iter(column.map(|val| match val {
                    ValueView::Uuid(u) => Some(u.to_string()),
                    ValueView::Null => None,
                    _ => panic!("expected uuid, got: {:?}", val),
                }))),
            ),
            DataType::Time(tu) => match tu {
                TimeUnit::Second => (
                    Field::new(
                        name,
                        ArrowDataType::Time64(ArrowTimeUnit::Microsecond),
                        true,
                    ),
                    Arc::new(Time64MicrosecondArray::from_iter(column.map(
                        |val| match val {
                            ValueView::Time(TimeUnit::Second, v) => Some(v * 1_000_000),
                            ValueView::Null => None,
                            _ => panic!("expected time in seconds, got: {:?}", val),
                        },
                    ))),
                ),
                TimeUnit::Millisecond => (
                    Field::new(
                        name,
                        ArrowDataType::Time64(ArrowTimeUnit::Microsecond),
                        true,
                    ),
                    Arc::new(Time64MicrosecondArray::from_iter(column.map(
                        |val| match val {
                            ValueView::Time(TimeUnit::Millisecond, v) => Some(v * 1000),
                            ValueView::Null => None,
                            _ => panic!("expected time in milliseconds, got: {:?}", val),
                        },
                    ))),
                ),
                TimeUnit::Microsecond => (
                    Field::new(
                        name,
                        ArrowDataType::Time64(ArrowTimeUnit::Microsecond),
                        true,
                    ),
                    Arc::new(Time64MicrosecondArray::from_iter(column.map(
                        |val| match val {
                            ValueView::Time(TimeUnit::Microsecond, v) => Some(v),
                            ValueView::Null => None,
                            _ => panic!("expected time in microseconds, got: {:?}", val),
                        },
                    ))),
                ),
                TimeUnit::Nanosecond => (
                    Field::new(name, ArrowDataType::Time64(ArrowTimeUnit::Nanosecond), true),
                    Arc::new(Time64NanosecondArray::from_iter(column.map(
                        |val| match val {
                            ValueView::Time(TimeUnit::Nanosecond, v) => Some(v),
                            ValueView::Null => None,
                            _ => panic!("expected time in nanoseconds, got: {:?}", val),
                        },
                    ))),
                ),
            },
            DataType::Date(tu) => {
                let field = Field::new(name, ArrowDataType::Date64, true);
                let arr = match tu {
                    TimeUnit::Second => {
                        Arc::new(Date64Array::from_iter(column.map(|val| match val {
                            ValueView::Date(_, d) => Some(d * 1000),
                            ValueView::Null => None,
                            _ => panic!("expected date, got: {:?}", val),
                        })))
                    }
                    TimeUnit::Millisecond => {
                        Arc::new(Date64Array::from_iter(column.map(|val| match val {
                            ValueView::Date(_, d) => Some(d),
                            ValueView::Null => None,
                            _ => panic!("expected date, got: {:?}", val),
                        })))
                    }
                    TimeUnit::Microsecond => {
                        Arc::new(Date64Array::from_iter(column.map(|val| match val {
                            ValueView::Date(_, d) => Some(d / 1000),
                            ValueView::Null => None,
                            _ => panic!("expected date, got: {:?}", val),
                        })))
                    }
                    TimeUnit::Nanosecond => {
                        Arc::new(Date64Array::from_iter(column.map(|val| match val {
                            ValueView::Date(_, d) => Some(d / 1_000_000),
                            ValueView::Null => None,
                            _ => panic!("expected date, got: {:?}", val),
                        })))
                    }
                };
                (field, arr)
            }
            DataType::TimeStamp(tu) => {
                let atu = into_arrow_timeunit(tu);
                match tu {
                    TimeUnit::Second => (
                        Field::new(name, ArrowDataType::Timestamp(atu, None), true),
                        Arc::new(TimestampSecondArray::from_iter(column.map(
                            |val| match val {
                                ValueView::TimeStamp(_tu, v) => Some(v),
                                ValueView::Null => None,
                                _ => panic!("expected timestamp, got: {:?}", val),
                            },
                        ))),
                    ),
                    TimeUnit::Millisecond => (
                        Field::new(name, ArrowDataType::Timestamp(atu, None), true),
                        Arc::new(TimestampMillisecondArray::from_iter(column.map(
                            |val| match val {
                                ValueView::TimeStamp(_tu, v) => Some(v),
                                ValueView::Null => None,
                                _ => panic!("expected timestamp, got: {:?}", val),
                            },
                        ))),
                    ),
                    TimeUnit::Microsecond => (
                        Field::new(name, ArrowDataType::Timestamp(atu, None), true),
                        Arc::new(TimestampMicrosecondArray::from_iter(column.map(
                            |val| match val {
                                ValueView::TimeStamp(_tu, v) => Some(v),
                                ValueView::Null => None,
                                _ => panic!("expected timestamp, got: {:?}", val),
                            },
                        ))),
                    ),
                    TimeUnit::Nanosecond => (
                        Field::new(name, ArrowDataType::Timestamp(atu, None), true),
                        Arc::new(TimestampNanosecondArray::from_iter(column.map(
                            |val| match val {
                                ValueView::TimeStamp(_tu, v) => Some(v),
                                ValueView::Null => None,
                                _ => panic!("expected timestamp, got: {:?}", val),
                            },
                        ))),
                    ),
                }
            }
            DataType::TimeStampUTC(tu) => {
                let atu = into_arrow_timeunit(tu);
                let tz: Arc<str> = Arc::from("+00:00");
                match tu {
                    TimeUnit::Second => (
                        Field::new(name, ArrowDataType::Timestamp(atu, Some(tz.clone())), true),
                        Arc::new(
                            TimestampSecondArray::from_iter(column.map(|val| match val {
                                ValueView::TimeStampUTC(_tu, v) => Some(v),
                                ValueView::Null => None,
                                _ => panic!("expected timestamp utc, got: {:?}", val),
                            }))
                            .with_timezone(tz),
                        ),
                    ),
                    TimeUnit::Millisecond => (
                        Field::new(name, ArrowDataType::Timestamp(atu, Some(tz.clone())), true),
                        Arc::new(
                            TimestampMillisecondArray::from_iter(column.map(|val| match val {
                                ValueView::TimeStampUTC(_tu, v) => Some(v),
                                ValueView::Null => None,
                                _ => panic!("expected timestamp utc, got: {:?}", val),
                            }))
                            .with_timezone(tz),
                        ),
                    ),
                    TimeUnit::Microsecond => (
                        Field::new(name, ArrowDataType::Timestamp(atu, Some(tz.clone())), true),
                        Arc::new(
                            TimestampMicrosecondArray::from_iter(column.map(|val| match val {
                                ValueView::TimeStampUTC(_tu, v) => Some(v),
                                ValueView::Null => None,
                                _ => panic!("expected timestamp utc, got: {:?}", val),
                            }))
                            .with_timezone(tz),
                        ),
                    ),
                    TimeUnit::Nanosecond => (
                        Field::new(name, ArrowDataType::Timestamp(atu, Some(tz.clone())), true),
                        Arc::new(
                            TimestampNanosecondArray::from_iter(column.map(|val| match val {
                                ValueView::TimeStampUTC(_tu, v) => Some(v),
                                ValueView::Null => None,
                                _ => panic!("expected timestamp utc, got: {:?}", val),
                            }))
                            .with_timezone(tz),
                        ),
                    ),
                }
            }
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

    Ok(ArrowRecordBatch::try_new(
        Arc::new(Schema::new(schema_columns)),
        rb_columns,
    )?)
}
