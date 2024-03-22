//! Test timestamps conversion from dataframe to arrow record batch

use arrow::array::{TimestampMillisecondArray, TimestampNanosecondArray, TimestampSecondArray};
use arrow::temporal_conversions::{
    timestamp_ms_to_datetime, timestamp_ns_to_datetime, timestamp_s_to_datetime,
    timestamp_us_to_datetime,
};
use arrow::{
    array::{ArrayRef, TimestampMicrosecondArray},
    datatypes::{DataType as ArrowDataType, Field, Schema, TimeUnit as ArrowTimeUnit},
    record_batch::RecordBatch as ArrowRecordBatch,
};
use arrow_msg::{df_to_recordbatch, RecordBatch};
use chrono::{DateTime, FixedOffset, Utc};
use quickcheck::TestResult;
use section::message::{Column, DataFrame, DataType, TimeUnit, ValueView};
use std::str::FromStr;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct Test {
    timestamps: Vec<i64>,
    with_timezone: bool,
}

impl Test {
    fn new(timestamps: Vec<i64>, with_timezone: bool) -> Self {
        Self {
            timestamps,
            with_timezone,
        }
    }
}

impl DataFrame for Test {
    fn columns(&self) -> Vec<section::message::Column<'_>> {
        match self.with_timezone {
            false => vec![
                Column::new(
                    "time_s",
                    DataType::TimeStamp(TimeUnit::Second),
                    Box::new(
                        self.timestamps
                            .iter()
                            .map(|v| ValueView::TimeStamp(TimeUnit::Second, *v)),
                    ),
                ),
                Column::new(
                    "time_ms",
                    DataType::TimeStamp(TimeUnit::Millisecond),
                    Box::new(
                        self.timestamps
                            .iter()
                            .map(|v| ValueView::TimeStamp(TimeUnit::Millisecond, *v)),
                    ),
                ),
                Column::new(
                    "time_us",
                    DataType::TimeStamp(TimeUnit::Microsecond),
                    Box::new(
                        self.timestamps
                            .iter()
                            .map(|v| ValueView::TimeStamp(TimeUnit::Microsecond, *v)),
                    ),
                ),
                Column::new(
                    "time_ns",
                    DataType::TimeStamp(TimeUnit::Nanosecond),
                    Box::new(
                        self.timestamps
                            .iter()
                            .map(|v| ValueView::TimeStamp(TimeUnit::Nanosecond, *v)),
                    ),
                ),
            ],
            true => vec![
                Column::new(
                    "time_s",
                    DataType::TimeStampUTC(TimeUnit::Second),
                    Box::new(
                        self.timestamps
                            .iter()
                            .map(|v| ValueView::TimeStampUTC(TimeUnit::Second, *v)),
                    ),
                ),
                Column::new(
                    "time_ms",
                    DataType::TimeStampUTC(TimeUnit::Millisecond),
                    Box::new(
                        self.timestamps
                            .iter()
                            .map(|v| ValueView::TimeStampUTC(TimeUnit::Millisecond, *v)),
                    ),
                ),
                Column::new(
                    "time_us",
                    DataType::TimeStampUTC(TimeUnit::Microsecond),
                    Box::new(
                        self.timestamps
                            .iter()
                            .map(|v| ValueView::TimeStampUTC(TimeUnit::Microsecond, *v)),
                    ),
                ),
                Column::new(
                    "time_ns",
                    DataType::TimeStampUTC(TimeUnit::Nanosecond),
                    Box::new(
                        self.timestamps
                            .iter()
                            .map(|v| ValueView::TimeStampUTC(TimeUnit::Nanosecond, *v)),
                    ),
                ),
            ],
        }
    }
}

// check values from dataframe timestamps are the same after conversion to arrow 'dataframe'
#[test]
fn test_timestamp_conv() {
    fn check(timestamps: Vec<i64>) -> TestResult {
        let df: Box<dyn DataFrame> = Box::new(Test::new(timestamps, false));
        let rb: RecordBatch = df_to_recordbatch(df.as_ref())
            .expect("failed to convert df to recorb batch")
            .into();
        let rb: Box<dyn DataFrame> = Box::new(rb);
        let df_columns = df.columns();
        let rb_columns = rb.columns();
        assert_eq!(df_columns.len(), rb_columns.len());
        assert_eq!(
            df_columns.iter().map(|c| c.name()).collect::<Vec<_>>(),
            rb_columns.iter().map(|c| c.name()).collect::<Vec<_>>(),
        );
        for (original, converted) in df_columns.into_iter().zip(rb_columns.into_iter()) {
            assert_eq!(original.collect::<Vec<_>>(), converted.collect::<Vec<_>>(),)
        }
        TestResult::passed()
    }
    quickcheck::quickcheck(check as fn(Vec<i64>) -> TestResult);
}

#[test]
fn test_timestamp_utc_conv() {
    fn check(timestamps: Vec<i64>) -> TestResult {
        let df: Box<dyn DataFrame> = Box::new(Test::new(timestamps, true));
        let rb: RecordBatch = df_to_recordbatch(df.as_ref()).unwrap().into();
        let rb: Box<dyn DataFrame> = Box::new(rb);
        let df_columns = df.columns();
        let rb_columns = rb.columns();
        assert_eq!(df_columns.len(), rb_columns.len());
        assert_eq!(
            df_columns.iter().map(|c| c.name()).collect::<Vec<_>>(),
            rb_columns.iter().map(|c| c.name()).collect::<Vec<_>>(),
        );
        for (original, converted) in df_columns.into_iter().zip(rb_columns.into_iter()) {
            assert_eq!(original.collect::<Vec<_>>(), converted.collect::<Vec<_>>(),)
        }
        TestResult::passed()
    }
    quickcheck::quickcheck(check as fn(Vec<i64>) -> TestResult);
}

// manual sanity check
// all timestamps with timezone are adjusted to UTC when reading values through DataFrame API
#[test]
fn test_timestamp_utc_offset_manual() {
    fn check(tz: &str, expected: i64) {
        let tz = Some(Arc::from(tz));
        let tz = move || tz.clone();
        let schema = Arc::new(Schema::new(vec![Field::new(
            "timestamp",
            ArrowDataType::Timestamp(ArrowTimeUnit::Second, tz()),
            true,
        )]));
        let timestamp: ArrayRef = Arc::new(
            TimestampSecondArray::from_iter(std::iter::once(Some(0))).with_timezone_opt(tz()),
        );
        let rb = ArrowRecordBatch::try_new(Arc::clone(&schema), vec![timestamp]).unwrap();
        let df: Box<dyn DataFrame> = Box::new(RecordBatch::new(rb));
        let column = df.columns().pop().unwrap();
        assert_eq!(
            column.collect::<Vec<_>>(),
            vec![ValueView::TimeStampUTC(TimeUnit::Second, expected)]
        )
    }
    check("-12:00", 43200);
    check("+12:00", -43200);
}

// check timestamps adjustment to UTC at ValueView level for all TimeUnits
#[test]
fn test_timestamps_utc_offset() {
    fn check(timestamps: Vec<i64>, hours: i8, minutes: u8) -> TestResult {
        // prevent chrono overflows
        // chrono datetypes are limited to +/- ~262,000 years
        let timestamps = timestamps
            .into_iter()
            .map(|v| match v {
                v if v < 0 => v % DateTime::<Utc>::MIN_UTC.timestamp(),
                v => v % DateTime::<Utc>::MAX_UTC.timestamp(),
            })
            .collect::<Vec<i64>>();
        let minutes = minutes % 60;
        let tz = match hours % 24 {
            hours if hours < 0 => format!("-{:02}:{:02}", hours.abs(), minutes),
            hours => format!("+{:02}:{:02}", hours.abs(), minutes),
        };
        let tz = Some(Arc::from(tz));
        let tz = move || tz.clone();

        let schema = Arc::new(Schema::new(vec![
            Field::new(
                "ts_s",
                ArrowDataType::Timestamp(ArrowTimeUnit::Second, tz()),
                true,
            ),
            Field::new(
                "ts_ms",
                ArrowDataType::Timestamp(ArrowTimeUnit::Millisecond, tz()),
                true,
            ),
            Field::new(
                "ts_us",
                ArrowDataType::Timestamp(ArrowTimeUnit::Microsecond, tz()),
                true,
            ),
            Field::new(
                "ts_ns",
                ArrowDataType::Timestamp(ArrowTimeUnit::Nanosecond, tz()),
                true,
            ),
        ]));
        let ts_s: ArrayRef = Arc::new(
            TimestampSecondArray::from_iter(timestamps.iter().copied().map(Some))
                .with_timezone_opt(tz()),
        );
        let ts_ms: ArrayRef = Arc::new(
            TimestampMillisecondArray::from_iter(timestamps.iter().copied().map(Some))
                .with_timezone_opt(tz()),
        );
        let ts_us: ArrayRef = Arc::new(
            TimestampMicrosecondArray::from_iter(timestamps.iter().copied().map(Some))
                .with_timezone_opt(tz()),
        );
        let ts_ns: ArrayRef = Arc::new(
            TimestampNanosecondArray::from_iter(timestamps.iter().copied().map(Some))
                .with_timezone_opt(tz()),
        );
        let rb = ArrowRecordBatch::try_new(Arc::clone(&schema), vec![ts_s, ts_ms, ts_us, ts_ns])
            .unwrap();
        let df: Box<dyn DataFrame> = Box::new(RecordBatch::new(rb));
        let offset = FixedOffset::from_str(tz().unwrap().as_ref()).unwrap();
        for column in df.columns() {
            let name = column.name().to_string();
            for (result, &original) in column.zip(timestamps.iter()) {
                match name.as_str() {
                    "ts_s" => {
                        let utc = timestamp_s_to_datetime(original)
                            .unwrap()
                            .and_local_timezone(offset)
                            .unwrap();
                        assert_eq!(
                            result,
                            ValueView::TimeStampUTC(TimeUnit::Second, utc.timestamp())
                        );
                    }
                    "ts_ms" => {
                        let utc = timestamp_ms_to_datetime(original)
                            .unwrap()
                            .and_local_timezone(offset)
                            .unwrap();
                        assert_eq!(
                            result,
                            ValueView::TimeStampUTC(TimeUnit::Millisecond, utc.timestamp_millis())
                        );
                    }
                    "ts_us" => {
                        let utc = timestamp_us_to_datetime(original)
                            .unwrap()
                            .and_local_timezone(offset)
                            .unwrap();
                        assert_eq!(
                            result,
                            ValueView::TimeStampUTC(TimeUnit::Microsecond, utc.timestamp_micros())
                        );
                    }
                    "ts_ns" => {
                        let utc = timestamp_ns_to_datetime(original)
                            .unwrap()
                            .and_local_timezone(offset)
                            .unwrap();
                        assert_eq!(
                            result,
                            ValueView::TimeStampUTC(
                                TimeUnit::Nanosecond,
                                utc.timestamp_nanos_opt().unwrap()
                            )
                        );
                    }
                    _ => unreachable!(),
                }
            }
        }
        TestResult::passed()
    }
    quickcheck::quickcheck(check as fn(Vec<i64>, hours: i8, minutes: u8) -> TestResult);
}
