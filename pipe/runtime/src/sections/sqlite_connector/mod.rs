pub mod destination;
pub mod source;

use std::sync::Arc;

use crate::message::RecordBatch;
use arrow::{
    array::{Array, ArrayRef, AsArray, BinaryArray, Float64Array, Int64Array, StringArray},
    datatypes::{
        DataType, Field, Float16Type, Float32Type, Float64Type, Int16Type, Int32Type, Int64Type,
        Int8Type, Schema,
    },
    error::ArrowError,
    record_batch::RecordBatch as _RecordBatch,
};
use sqlite_connector::{ColumnType, SqlitePayload, Value};

fn to_datatype(sqlite_coltype: ColumnType) -> DataType {
    // FIXME: make use of int32/f32 where possible?
    match sqlite_coltype {
        ColumnType::Int => DataType::Int64,
        ColumnType::Blob => DataType::Binary,
        ColumnType::Text => DataType::Utf8,
        ColumnType::Real => DataType::Float64,
        _ => panic!("unexpected sqlite type: {:?}", sqlite_coltype),
    }
}

fn to_coltype(datatype: &DataType) -> ColumnType {
    match datatype {
        DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64 => ColumnType::Int,
        DataType::Binary | DataType::LargeBinary => ColumnType::Blob,
        DataType::Float16 | DataType::Float32 | DataType::Float64 => ColumnType::Real,
        DataType::Utf8 | DataType::LargeUtf8 => ColumnType::Text,
        DataType::Boolean => ColumnType::Bool,
        _ => unimplemented!("Arrow DataType '{}'", datatype),
    }
}

impl TryInto<RecordBatch> for &SqlitePayload {
    // FIXME: proper conv error type
    type Error = ArrowError;

    fn try_into(self) -> Result<RecordBatch, Self::Error> {
        // FIXME: schema allocated on every conversion
        let schema = Arc::new(Schema::new(
            self.columns
                .iter()
                .zip(self.column_types.iter())
                .map(|(col_name, col_type)| Field::new(col_name, to_datatype(*col_type), true))
                .collect::<Vec<Field>>(),
        ));
        let columns: Vec<ArrayRef> = self
            .values
            .iter()
            .zip(self.column_types.iter())
            .map(|(column, column_type)| {
                // FIXME proper conversion: replace this match
                match column_type {
                    ColumnType::Int => {
                        let arrow_column = column
                            .iter()
                            .map(|col| match col {
                                Value::Int(v) => Some(*v),
                                Value::Null => None,
                                _ => unreachable!(),
                            })
                            .collect::<Int64Array>();
                        Arc::new(arrow_column) as Arc<dyn Array>
                    }
                    ColumnType::Blob => {
                        let arrow_column = column
                            .iter()
                            .map(|col| match col {
                                Value::Blob(b) => Some(b.clone()),
                                Value::Null => None,
                                _ => unreachable!(),
                            })
                            .collect::<BinaryArray>();
                        Arc::new(arrow_column) as Arc<dyn Array>
                    }
                    ColumnType::Text => {
                        let arrow_column = column
                            .iter()
                            .map(|col| match col {
                                Value::Text(s) => Some(s.clone()),
                                Value::Null => None,
                                _ => unreachable!(),
                            })
                            .collect::<StringArray>();
                        Arc::new(arrow_column) as Arc<dyn Array>
                    }
                    ColumnType::Real => {
                        let arrow_column = column
                            .iter()
                            .map(|col| match col {
                                Value::Real(f) => Some(*f),
                                Value::Null => None,
                                _ => unreachable!(),
                            })
                            .collect::<Float64Array>();
                        Arc::new(arrow_column)
                    }
                    // FIXME:
                    _ => unreachable!(),
                }
            })
            .collect();
        _RecordBatch::try_new(schema, columns).map(RecordBatch)
    }
}

impl TryInto<RecordBatch> for SqlitePayload {
    // FIXME: proper conv error type
    type Error = ArrowError;

    fn try_into(self) -> Result<RecordBatch, Self::Error> {
        (&self).try_into()
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct SqlitePayloadNewType(SqlitePayload);

impl From<&RecordBatch> for SqlitePayloadNewType {
    fn from(arrow_rb: &RecordBatch) -> Self {
        let (columns, column_types) = arrow_rb.schema().fields.into_iter().fold(
            (vec![], vec![]),
            |(mut columns, mut column_types), field| {
                columns.push(field.name().clone());
                column_types.push(to_coltype(field.data_type()));
                (columns, column_types)
            },
        );
        let values = arrow_rb
            .columns()
            .iter()
            .map(|column| {
                let array = column.as_ref();
                // FIXME: is it possible to use downcast_macro from arrow?
                match array.data_type() {
                    DataType::Int8 => array
                        .as_primitive::<Int8Type>()
                        .into_iter()
                        .map(|x| x.map(|x| x as i64).map(Value::Int).unwrap_or(Value::Null))
                        .collect(),
                    DataType::Int16 => array
                        .as_primitive::<Int16Type>()
                        .into_iter()
                        .map(|x| x.map(|x| x as i64).map(Value::Int).unwrap_or(Value::Null))
                        .collect(),
                    DataType::Int32 => array
                        .as_primitive::<Int32Type>()
                        .into_iter()
                        .map(|x| x.map(|x| x as i64).map(Value::Int).unwrap_or(Value::Null))
                        .collect(),
                    DataType::Int64 => array
                        .as_primitive::<Int64Type>()
                        .into_iter()
                        .map(|x| x.map(Value::Int).unwrap_or(Value::Null))
                        .collect(),
                    DataType::Float16 => array
                        .as_primitive::<Float16Type>()
                        .into_iter()
                        .map(|x| x.map(|x| x.into()).map(Value::Real).unwrap_or(Value::Null))
                        .collect(),
                    DataType::Float32 => array
                        .as_primitive::<Float32Type>()
                        .into_iter()
                        .map(|x| x.map(|x| x.into()).map(Value::Real).unwrap_or(Value::Null))
                        .collect(),
                    DataType::Float64 => array
                        .as_primitive::<Float64Type>()
                        .into_iter()
                        .map(|x| x.map(Value::Real).unwrap_or(Value::Null))
                        .collect(),
                    DataType::Binary => array
                        .as_binary::<i32>()
                        .into_iter()
                        .map(|x| x.map(|x| Value::Blob(x.into())).unwrap_or(Value::Null))
                        .collect(),
                    DataType::Utf8 => array
                        .as_string::<i32>()
                        .into_iter()
                        .map(|x| x.map(|x| Value::Text(x.into())).unwrap_or(Value::Null))
                        .collect(),
                    DataType::Boolean => array
                        .as_boolean()
                        .into_iter()
                        .map(|x| x.map(Value::Bool).unwrap_or(Value::Null))
                        .collect(),
                    dt => unimplemented!("unimplemented {}", dt),
                }
            })
            .collect();
        let payload = SqlitePayload {
            columns: Arc::from(columns),
            column_types: Arc::from(column_types),
            values,
            // FIXME:
            offset: 0,
        };
        SqlitePayloadNewType(payload)
    }
}
