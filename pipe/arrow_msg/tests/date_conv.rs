// Check date conversions

use std::sync::Arc;

use arrow::{
    array::{ArrayRef, Date32Array, Date64Array},
    datatypes::{DataType as ArrowDataType, Field, Schema},
    record_batch::RecordBatch as ArrowRecordBatch,
};
use arrow_msg::{df_to_recordbatch, RecordBatch};
use quickcheck::TestResult;
use section::message::{Column, DataFrame, DataType, TimeUnit, ValueView};

// test arrow's Date32 to ValueView
#[test]
fn test_date32_conv() {
    fn check(dates: Vec<i32>) -> TestResult {
        let schema = Arc::new(Schema::new(vec![Field::new(
            "date32",
            ArrowDataType::Date32,
            true,
        )]));
        let date32: ArrayRef = Arc::new(Date32Array::from_iter(dates.iter().copied().map(Some)));
        let rb = ArrowRecordBatch::try_new(Arc::clone(&schema), vec![date32]).unwrap();
        let df: Box<dyn DataFrame> = Box::new(RecordBatch::new(rb));
        for column in df.columns() {
            let name = column.name().to_string();
            for (result, &days) in column.zip(dates.iter()) {
                match name.as_str() {
                    "date32" => {
                        let seconds = days as i64 * 86400;
                        assert_eq!(result, ValueView::Date(TimeUnit::Second, seconds))
                    }
                    _ => unreachable!(),
                }
            }
        }
        TestResult::passed()
    }
    quickcheck::quickcheck(check as fn(Vec<i32>) -> TestResult);
}

// test arrow's Date64 conversion to ValueView
#[test]
fn test_date64_conv() {
    fn check(milliseconds: Vec<i64>) -> TestResult {
        let schema = Arc::new(Schema::new(vec![Field::new(
            "date64",
            ArrowDataType::Date64,
            true,
        )]));
        let date64: ArrayRef = Arc::new(Date64Array::from_iter(
            milliseconds.iter().copied().map(Some),
        ));
        let rb = ArrowRecordBatch::try_new(Arc::clone(&schema), vec![date64]).unwrap();
        let df: Box<dyn DataFrame> = Box::new(RecordBatch::new(rb));
        for column in df.columns() {
            let name = column.name().to_string();
            for (result, &millis) in column.zip(milliseconds.iter()) {
                match name.as_str() {
                    "date64" => assert_eq!(result, ValueView::Date(TimeUnit::Millisecond, millis)),
                    _ => unreachable!(),
                }
            }
        }
        TestResult::passed()
    }
    quickcheck::quickcheck(check as fn(Vec<i64>) -> TestResult);
}

#[derive(Debug)]
struct Test {
    dates: Vec<i64>,
}

impl DataFrame for Test {
    fn columns(&self) -> Vec<section::message::Column<'_>> {
        vec![
            Column::new(
                "date_s",
                DataType::Date(TimeUnit::Second),
                Box::new(
                    self.dates
                        .iter()
                        .map(|v| ValueView::Date(TimeUnit::Second, *v)),
                ),
            ),
            Column::new(
                "date_ms",
                DataType::Date(TimeUnit::Millisecond),
                Box::new(
                    self.dates
                        .iter()
                        .map(|v| ValueView::Date(TimeUnit::Millisecond, *v)),
                ),
            ),
            Column::new(
                "date_us",
                DataType::Date(TimeUnit::Microsecond),
                Box::new(
                    self.dates
                        .iter()
                        .map(|v| ValueView::Date(TimeUnit::Microsecond, *v)),
                ),
            ),
            Column::new(
                "date_ns",
                DataType::Date(TimeUnit::Nanosecond),
                Box::new(
                    self.dates
                        .iter()
                        .map(|v| ValueView::Date(TimeUnit::Nanosecond, *v)),
                ),
            ),
        ]
    }
}

// test conversion from dataframe to arrow record batch
#[test]
fn test_date_roundtrip_conv() {
    fn check(dates: Vec<i64>) -> TestResult {
        // prevent overflows
        let dates: Vec<i64> = dates
            .into_iter()
            .map(|v| {
                match (v.is_positive() && v >= i64::MAX / 1000)
                    || (v.is_negative() && v <= i64::MIN / 1000)
                {
                    true => v / 1000,
                    false => v,
                }
            })
            .collect();
        let df: Box<dyn DataFrame> = Box::new(Test {
            dates: dates.clone(),
        });
        let rb: Box<dyn DataFrame> =
            Box::new(RecordBatch::new(df_to_recordbatch(df.as_ref()).unwrap()));
        let df_columns = df.columns();
        let rb_columns = rb.columns();
        assert_eq!(df_columns.len(), rb_columns.len());
        for (original, converted) in df_columns.into_iter().zip(rb_columns.into_iter()) {
            for (o, c) in original.zip(converted) {
                let cv = match c {
                    ValueView::Date(TimeUnit::Millisecond, cv) => cv,
                    _ => unreachable!("converted values expected in milliseconds, got {:?}", c),
                };
                match o {
                    ValueView::Date(TimeUnit::Second, v) => assert_eq!(v * 1000, cv),
                    ValueView::Date(TimeUnit::Millisecond, v) => assert_eq!(v, cv),
                    ValueView::Date(TimeUnit::Microsecond, v) => assert_eq!(v / 1000, cv),
                    ValueView::Date(TimeUnit::Nanosecond, v) => assert_eq!(v / 1_000_000, cv),
                    _ => unreachable!(),
                }
            }
        }
        TestResult::passed()
    }
    quickcheck::quickcheck(check as fn(Vec<i64>) -> TestResult)
}
