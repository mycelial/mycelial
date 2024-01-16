use arrow::{
    array::{Array, Float64Array, Int32Array, StringArray, UnionArray},
    buffer::Buffer,
    datatypes::{DataType as ArrowDataType, Field, Schema, UnionFields, UnionMode},
    record_batch::RecordBatch as ArrowRecordBatch,
};
use arrow_msg::RecordBatch;
use quickcheck::TestResult;
use section::message::{DataFrame, ValueView};
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
        let mut hashset = HashSet::new();
        let mut prng = XorShift64::new(seed);
        while hashset.len() < 3 {
            hashset.insert(((prng.next() % 127) as i8).abs());
        }
        let type_ids: Vec<i8> = hashset.iter().copied().collect();
        let int_array = Int32Array::from(vec![1, 34]);
        let float_array = Float64Array::from(vec![3.2]);
        let string_array = StringArray::from(vec!["one", "two"]);
        let type_id_buffer = Buffer::from_slice_ref([
            type_ids[0],
            type_ids[1],
            type_ids[0],
            type_ids[2],
            type_ids[2],
        ]);
        let value_offsets_buffer = Buffer::from_slice_ref([0_i32, 0, 1, 0, 1]);

        let children: Vec<(Field, Arc<dyn Array>)> = vec![
            (
                Field::new("i32", ArrowDataType::Int32, true),
                Arc::new(int_array),
            ),
            (
                Field::new("f64", ArrowDataType::Float64, true),
                Arc::new(float_array),
            ),
            (
                Field::new("str", ArrowDataType::Utf8, true),
                Arc::new(string_array),
            ),
        ];

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
                ValueView::I32(1),
                ValueView::F64(3.2),
                ValueView::I32(34),
                ValueView::Str("one"),
                ValueView::Str("two")
            ],
        );
        TestResult::passed()
    }
    quickcheck::quickcheck(check as fn(u64) -> TestResult);
}
