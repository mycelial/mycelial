use arrow::{
    array::{
        Array, ArrayRef, BinaryArray, Date32Array, Date64Array, Float32Array, Float64Array,
        Int16Array, Int32Array, Int64Array, Int8Array, StringArray, Time32MillisecondArray,
        Time32SecondArray, Time64MicrosecondArray, Time64NanosecondArray,
        TimestampMicrosecondArray, TimestampMillisecondArray, TimestampNanosecondArray,
        TimestampSecondArray, UInt16Array, UInt32Array, UInt64Array, UInt8Array, UnionArray,
    },
    buffer::Buffer,
    datatypes::{DataType as ArrowDataType, Field, Schema, UnionFields, UnionMode},
    record_batch::RecordBatch as ArrowRecordBatch,
};
use arrow_msg::RecordBatch;
use quickcheck::TestResult;
use section::message::{DataFrame, TimeUnit, ValueView};
use std::{collections::HashSet, sync::Arc};

pub struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(state: u64) -> Self {
        Self {
            state: state.max(1),
        }
    }
    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }
}

// Goal of this test is to check that union array type_id is insignificant.
// In order to do so we will create union array with randomly assigned type_id
// Derived dataframe should output exactly same values of any set of type_ids
#[test]
fn test_union_array_iter() {
    fn check(seed: u64) -> TestResult {
        let arrays: Vec<ArrayRef> = vec![
            Arc::new(Int8Array::from(vec![-8])),
            Arc::new(Int16Array::from(vec![-16])),
            Arc::new(Int32Array::from(vec![-32])),
            Arc::new(Int64Array::from(vec![-64])),
            Arc::new(UInt8Array::from(vec![8])),
            Arc::new(UInt16Array::from(vec![16])),
            Arc::new(UInt32Array::from(vec![32])),
            Arc::new(UInt64Array::from(vec![64])),
            Arc::new(Float32Array::from(vec![3.2])),
            Arc::new(Float64Array::from(vec![6.4])),
            Arc::new(Time32SecondArray::from(vec![100])),
            Arc::new(Time32MillisecondArray::from(vec![200])),
            Arc::new(Time64MicrosecondArray::from(vec![300])),
            Arc::new(Time64NanosecondArray::from(vec![400])),
            Arc::new(Date32Array::from(vec![1])),
            Arc::new(Date64Array::from(vec![2])),
            Arc::new(TimestampSecondArray::from(vec![1])),
            Arc::new(TimestampMillisecondArray::from(vec![2])),
            Arc::new(TimestampMicrosecondArray::from(vec![3])),
            Arc::new(TimestampNanosecondArray::from(vec![4])),
            Arc::new(StringArray::from(vec!["one"])),
            Arc::new(BinaryArray::from(vec![b"bin".as_slice()])),
            // FIXME: add decimals
        ];

        let mut hashset = HashSet::new();
        let mut prng = XorShift64::new(seed);
        while hashset.len() < arrays.len() {
            hashset.insert(((prng.next() % 127) as i8).abs());
        }
        let type_ids: Vec<i8> = hashset.iter().copied().collect();

        let type_id_buffer = Buffer::from_slice_ref(type_ids.as_slice());
        let value_offsets_buffer = Buffer::from_iter((0..type_ids.len()).map(|_| 0));

        let children: Vec<(Field, Arc<dyn Array>)> = arrays
            .into_iter()
            .map(|array| {
                let dt = array.data_type();
                let field = Field::new(format!("{:?}", dt), dt.clone(), true);
                (field, array)
            })
            .collect();

        let fields: Vec<_> = children.iter().map(|(f, _)| f.clone()).collect();

        let array = UnionArray::try_new(
            &type_ids,
            type_id_buffer,
            Some(value_offsets_buffer),
            children,
        )
        .unwrap();
        let union_dt = ArrowDataType::Union(UnionFields::new(type_ids, fields), UnionMode::Dense);
        let record_batch = ArrowRecordBatch::try_new(
            Arc::new(Schema::new(vec![Field::new("union_test", union_dt, true)])),
            vec![Arc::new(array)],
        )
        .unwrap();

        let df: Box<dyn DataFrame> = Box::new(RecordBatch::new(record_batch));
        let mut columns = df.columns();
        assert_eq!(columns.len(), 1);
        let column = columns.pop().unwrap();
        assert_eq!(column.name(), "union_test");

        assert_eq!(
            column.collect::<Vec<ValueView>>(),
            vec![
                ValueView::I8(-8),
                ValueView::I16(-16),
                ValueView::I32(-32),
                ValueView::I64(-64),
                ValueView::U8(8),
                ValueView::U16(16),
                ValueView::U32(32),
                ValueView::U64(64),
                ValueView::F32(3.2),
                ValueView::F64(6.4),
                ValueView::Time(TimeUnit::Second, 100),
                ValueView::Time(TimeUnit::Millisecond, 200),
                ValueView::Time(TimeUnit::Microsecond, 300),
                ValueView::Time(TimeUnit::Nanosecond, 400),
                ValueView::Date(TimeUnit::Second, 86400),
                ValueView::Date(TimeUnit::Millisecond, 2),
                ValueView::TimeStamp(TimeUnit::Second, 1),
                ValueView::TimeStamp(TimeUnit::Millisecond, 2),
                ValueView::TimeStamp(TimeUnit::Microsecond, 3),
                ValueView::TimeStamp(TimeUnit::Nanosecond, 4),
                ValueView::Str("one"),
                ValueView::Bin(b"bin".as_slice())
            ],
        );
        TestResult::passed()
    }
    quickcheck::quickcheck(check as fn(u64) -> TestResult);
}
