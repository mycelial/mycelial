// Check time conversions

use std::sync::Arc;

use arrow::{
    array::{
        ArrayRef, Time32MillisecondArray, Time32SecondArray, Time64MicrosecondArray,
        Time64NanosecondArray,
    },
    datatypes::{DataType as ArrowDataType, Field, Schema, TimeUnit as ArrowTimeUnit},
    record_batch::RecordBatch as ArrowRecordBatch,
};
use arrow_msg::{df_to_recordbatch, RecordBatch};
use quickcheck::TestResult;
use section::message::{Column, DataFrame, DataType, TimeUnit, ValueView};

// test arrow's Time32 to ValueView
#[test]
fn test_time32_conv() {
    fn check(times: Vec<i32>) -> TestResult {
        let schema = Arc::new(Schema::new(vec![
            Field::new(
                "time32_s",
                ArrowDataType::Time32(ArrowTimeUnit::Second),
                true,
            ),
            Field::new(
                "time32_ms",
                ArrowDataType::Time32(ArrowTimeUnit::Millisecond),
                true,
            ),
        ]));
        let time32_s: ArrayRef = Arc::new(Time32SecondArray::from_iter(
            times.iter().copied().map(Some),
        ));
        let time32_ms: ArrayRef = Arc::new(Time32MillisecondArray::from_iter(
            times.iter().copied().map(Some),
        ));
        let rb = ArrowRecordBatch::try_new(Arc::clone(&schema), vec![time32_s, time32_ms]).unwrap();
        let df: Box<dyn DataFrame> = Box::new(RecordBatch::new(rb));
        for column in df.columns() {
            let name = column.name().to_string();
            for (result, &seconds) in column.zip(times.iter()) {
                let seconds = seconds as i64;
                match name.as_str() {
                    "time32_s" => assert_eq!(result, ValueView::Time(TimeUnit::Second, seconds)),
                    "time32_ms" => {
                        assert_eq!(result, ValueView::Time(TimeUnit::Millisecond, seconds))
                    }
                    _ => unreachable!(),
                }
            }
        }
        TestResult::passed()
    }
    quickcheck::quickcheck(check as fn(Vec<i32>) -> TestResult);
}

// test arrow's Time64 conversion to ValueView
#[test]
fn test_time64_conv() {
    fn check(times: Vec<i64>) -> TestResult {
        let schema = Arc::new(Schema::new(vec![
            Field::new(
                "time64_us",
                ArrowDataType::Time64(ArrowTimeUnit::Microsecond),
                true,
            ),
            Field::new(
                "time64_ns",
                ArrowDataType::Time64(ArrowTimeUnit::Nanosecond),
                true,
            ),
        ]));
        let time64_us: ArrayRef = Arc::new(Time64MicrosecondArray::from_iter(
            times.iter().copied().map(Some),
        ));
        let time64_ns: ArrayRef = Arc::new(Time64NanosecondArray::from_iter(
            times.iter().copied().map(Some),
        ));
        let rb =
            ArrowRecordBatch::try_new(Arc::clone(&schema), vec![time64_us, time64_ns]).unwrap();
        let df: Box<dyn DataFrame> = Box::new(RecordBatch::new(rb));
        for column in df.columns() {
            let name = column.name().to_string();
            for (result, &time) in column.zip(times.iter()) {
                match name.as_str() {
                    "time64_us" => assert_eq!(result, ValueView::Time(TimeUnit::Microsecond, time)),
                    "time64_ns" => assert_eq!(result, ValueView::Time(TimeUnit::Nanosecond, time)),
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
    times: Vec<i64>,
}

impl DataFrame for Test {
    fn columns(&self) -> Vec<section::message::Column<'_>> {
        vec![
            Column::new(
                "time_s",
                DataType::Time(TimeUnit::Second),
                Box::new(
                    self.times
                        .iter()
                        .map(|v| ValueView::Time(TimeUnit::Second, *v)),
                ),
            ),
            Column::new(
                "time_ms",
                DataType::Time(TimeUnit::Millisecond),
                Box::new(
                    self.times
                        .iter()
                        .map(|v| ValueView::Time(TimeUnit::Millisecond, *v)),
                ),
            ),
            Column::new(
                "time_us",
                DataType::Time(TimeUnit::Microsecond),
                Box::new(
                    self.times
                        .iter()
                        .map(|v| ValueView::Time(TimeUnit::Microsecond, *v)),
                ),
            ),
            Column::new(
                "time_ns",
                DataType::Time(TimeUnit::Nanosecond),
                Box::new(
                    self.times
                        .iter()
                        .map(|v| ValueView::Time(TimeUnit::Nanosecond, *v)),
                ),
            ),
        ]
    }
}

#[test]
fn test_time_roundtrip_conv() {
    fn check(times: Vec<i64>) -> TestResult {
        // prevent overflows
        let times: Vec<i64> = times
            .into_iter()
            .map(|v| {
                match (v.is_positive() && v >= i64::MAX / 1_000_000)
                    || (v.is_negative() && v <= i64::MIN / 1_000_000)
                {
                    true => v / 1_000_000,
                    false => v,
                }
            })
            .collect();
        let df: Box<dyn DataFrame> = Box::new(Test {
            times: times.clone(),
        });
        let rb: Box<dyn DataFrame> =
            Box::new(RecordBatch::new(df_to_recordbatch(df.as_ref()).unwrap()));
        let df_columns = df.columns();
        let rb_columns = rb.columns();
        assert_eq!(df_columns.len(), rb_columns.len());
        for (original, converted) in df_columns.into_iter().zip(rb_columns.into_iter()) {
            for (o, c) in original.zip(converted) {
                match (o, c) {
                    (
                        ValueView::Time(TimeUnit::Second, ov),
                        ValueView::Time(TimeUnit::Microsecond, cv),
                    ) => {
                        assert_eq!(ov * 1_000_000, cv)
                    }
                    (
                        ValueView::Time(TimeUnit::Millisecond, ov),
                        ValueView::Time(TimeUnit::Microsecond, cv),
                    ) => {
                        assert_eq!(ov * 1000, cv)
                    }
                    (
                        ValueView::Time(TimeUnit::Microsecond, ov),
                        ValueView::Time(TimeUnit::Microsecond, cv),
                    ) => {
                        assert_eq!(ov, cv)
                    }
                    (
                        ValueView::Time(TimeUnit::Nanosecond, ov),
                        ValueView::Time(TimeUnit::Nanosecond, cv),
                    ) => {
                        assert_eq!(ov, cv)
                    }
                    _ => unreachable!(),
                }
            }
        }
        TestResult::passed()
    }
    quickcheck::quickcheck(check as fn(Vec<i64>) -> TestResult)
}
