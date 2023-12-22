use std::{collections::HashMap, sync::Arc};

use arrow::{
    array::{
        as_union_array, Array, ArrayBuilder, ArrayRef, AsArray, BinaryArray, BinaryBuilder,
        BooleanArray, BooleanBuilder, Date64Array, Date64Builder, Decimal128Array,
        Decimal128Builder, Float32Array, Float32Builder, Float64Array, Float64Builder, Int16Array,
        Int16Builder, Int32Array, Int32Builder, Int64Array, Int64Builder, Int8Array, Int8Builder,
        NullBuilder, StringArray, StringBuilder, Time64MicrosecondArray, Time64MicrosecondBuilder,
        TimestampMicrosecondArray, TimestampMicrosecondBuilder, UInt16Array, UInt16Builder,
        UInt32Array, UInt32Builder, UInt64Array, UInt64Builder, UInt8Array, UInt8Builder,
        UnionArray,
    },
    buffer::Buffer,
    datatypes::{
        DataType as ArrowDataType, Date64Type, Decimal128Type, Field, Float32Type, Float64Type,
        Int16Type, Int32Type, Int64Type, Int8Type, Schema, SchemaRef, Time64MicrosecondType,
        TimeUnit, TimestampMicrosecondType, UInt16Type, UInt32Type, UInt64Type, UInt8Type,
        UnionFields, UnionMode, DECIMAL128_MAX_PRECISION, DECIMAL_DEFAULT_SCALE,
    },
    record_batch::RecordBatch as ArrowRecordBatch,
};
use section::{
    decimal::Decimal,
    message::{Ack, Chunk, Column, DataFrame, DataType, Message, ValueView},
    SectionError,
};

// Wrap around arrow record batch, which implements dataframe
#[derive(Debug)]
pub struct RecordBatch {
    inner: ArrowRecordBatch,
    schema: SchemaRef,
}

impl RecordBatch {
    pub fn new(inner: ArrowRecordBatch, schema: SchemaRef) -> Self {
        Self { inner, schema }
    }
}

impl From<ArrowRecordBatch> for RecordBatch {
    fn from(value: ArrowRecordBatch) -> Self {
        let schema = value.schema();
        Self::new(value, schema)
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
    let iter = (0..array.len()).map(|index| {
        let field_map: HashMap<i8, &Arc<Field>> = union_fields.iter().collect();
        let type_id = array.type_id(index);
        let value_offset = array.value_offset(index);
        let child = array.child(type_id);
        match DataType::from(type_id) {
            DataType::Null => ValueView::Null,
            DataType::I8 => ValueView::I8(child.as_primitive::<Int8Type>().value(value_offset)),
            DataType::I16 => ValueView::I16(child.as_primitive::<Int16Type>().value(value_offset)),
            DataType::I32 => ValueView::I32(child.as_primitive::<Int32Type>().value(value_offset)),
            DataType::I64 => ValueView::I64(child.as_primitive::<Int64Type>().value(value_offset)),
            DataType::U8 => ValueView::U8(child.as_primitive::<UInt8Type>().value(value_offset)),
            DataType::U16 => ValueView::U16(child.as_primitive::<UInt16Type>().value(value_offset)),
            DataType::U32 => ValueView::U32(child.as_primitive::<UInt32Type>().value(value_offset)),
            DataType::U64 => ValueView::U64(child.as_primitive::<UInt64Type>().value(value_offset)),
            DataType::F32 => {
                ValueView::F32(child.as_primitive::<Float32Type>().value(value_offset))
            }
            DataType::F64 => {
                ValueView::F64(child.as_primitive::<Float64Type>().value(value_offset))
            }
            DataType::Str => ValueView::Str(child.as_string::<i32>().value(value_offset)),
            DataType::Bin => ValueView::Bin(child.as_binary::<i32>().value(value_offset)),
            DataType::Decimal => ValueView::Decimal({
                let d = child.as_primitive::<Decimal128Type>().value(value_offset);
                let field = *field_map.get(&type_id).unwrap();
                if let ArrowDataType::Decimal128(_, scale) = field.data_type() {
                    Decimal::from_i128_with_scale(d, *scale as _)
                } else {
                    panic!(
                        "expected field data type to be Decimal128, got {:?} instead",
                        field
                    )
                }
            }),
            DataType::Time => ValueView::Time(
                child
                    .as_primitive::<Time64MicrosecondType>()
                    .value(value_offset),
            ),
            DataType::Date => {
                ValueView::Time(child.as_primitive::<Date64Type>().value(value_offset))
            }
            DataType::TimeStamp => ValueView::TimeStamp(
                child
                    .as_primitive::<TimestampMicrosecondType>()
                    .value(value_offset),
            ),
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
                    ArrowDataType::Time64(_tu) => {
                        let arr = column.as_primitive::<Time64MicrosecondType>();
                        (
                            DataType::Time,
                            Box::new(
                                arr.iter()
                                    .map(|val| val.map(ValueView::Time).unwrap_or(ValueView::Null)),
                            ),
                        )
                    }
                    ArrowDataType::Date64 => {
                        let arr = column.as_primitive::<Date64Type>();
                        (
                            DataType::Date,
                            Box::new(
                                arr.iter()
                                    .map(|val| val.map(ValueView::Date).unwrap_or(ValueView::Null)),
                            ),
                        )
                    }
                    ArrowDataType::Timestamp(_tu, _tz) => {
                        let arr = column.as_primitive::<TimestampMicrosecondType>();
                        (
                            DataType::TimeStamp,
                            Box::new(arr.iter().map(|val| {
                                val.map(ValueView::TimeStamp).unwrap_or(ValueView::Null)
                            })),
                        )
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
        DataType::Time => ArrowDataType::Time64(TimeUnit::Microsecond),
        DataType::Date => ArrowDataType::Date64,
        DataType::TimeStamp => ArrowDataType::Timestamp(TimeUnit::Microsecond, None),
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

pub fn df_to_recordbatch(df: Box<dyn DataFrame>) -> Result<ArrowRecordBatch, SectionError> {
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
            DataType::Time => (
                Field::new(name, ArrowDataType::Time64(TimeUnit::Microsecond), true),
                Arc::new(Time64MicrosecondArray::from_iter(column.map(
                    |val| match val {
                        ValueView::Time(v) => Some(v),
                        ValueView::Null => None,
                        _ => panic!("expected time, got: {:?}", val),
                    },
                ))),
            ),
            DataType::Date => (
                Field::new(name, ArrowDataType::Date64, true),
                Arc::new(Date64Array::from_iter(column.map(|val| match val {
                    ValueView::Date(d) => Some(d / 1000), // micros to millis
                    ValueView::Null => None,
                    _ => panic!("expected date, got: {:?}", val),
                }))),
            ),
            DataType::TimeStamp => (
                Field::new(
                    name,
                    ArrowDataType::Timestamp(TimeUnit::Microsecond, None),
                    true,
                ),
                Arc::new(TimestampMicrosecondArray::from_iter(column.map(
                    |val| match val {
                        ValueView::TimeStamp(v) => Some(v),
                        ValueView::Null => None,
                        _ => panic!("expected timestamp, got: {:?}", val),
                    },
                ))),
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

    Ok(ArrowRecordBatch::try_new(
        Arc::new(Schema::new(schema_columns)),
        rb_columns,
    )?)
}
