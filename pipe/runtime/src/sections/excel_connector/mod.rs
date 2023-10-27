pub mod source;

use std::sync::Arc;

use crate::message::RecordBatch;
use arrow::{
    array::{
        Array, ArrayRef, BinaryArray, BooleanArray, Float64Array, Int64Array, StringArray,
        TimestampSecondArray,
    },
    datatypes::{ArrowTimestampType, DataType, Field, Schema, TimeUnit, TimestampSecondType},
    error::ArrowError,
    record_batch::RecordBatch as _RecordBatch,
};
use excel_connector::{ColumnType, ExcelPayload, Value};

// convert excel column type to arrow data type
fn to_datatype(excel_coltype: ColumnType) -> DataType {
    // FIXME: make use of int32/f32 where possible?
    match excel_coltype {
        ColumnType::Int => DataType::Int64,
        ColumnType::Blob => DataType::Binary,
        ColumnType::Text => DataType::Utf8,
        ColumnType::Real => DataType::Float64,
        ColumnType::Numeric => DataType::Float64,
        ColumnType::Bool => DataType::Boolean,
        ColumnType::DateTime => DataType::Timestamp(TimeUnit::Second, None),
        _ => unimplemented!("Arrow DataType '{}'", excel_coltype),
    }
}

impl TryInto<RecordBatch> for &ExcelPayload {
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
                    ColumnType::Bool => {
                        let arrow_column = column
                            .iter()
                            .map(|col| match col {
                                Value::Bool(b) => Some(*b),
                                Value::Null => None,
                                _ => unreachable!(),
                            })
                            .collect::<BooleanArray>();
                        Arc::new(arrow_column)
                    }
                    ColumnType::DateTime => {
                        let arrow_column = column
                            .iter()
                            .map(|col| match col {
                                Value::DateTime(d) => TimestampSecondType::make_value(*d),
                                Value::Null => None,
                                _ => unreachable!(),
                            })
                            .collect::<TimestampSecondArray>();
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

impl TryInto<RecordBatch> for ExcelPayload {
    // FIXME: proper conv error type
    type Error = ArrowError;

    fn try_into(self) -> Result<RecordBatch, Self::Error> {
        (&self).try_into()
    }
}