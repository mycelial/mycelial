//! Section messaging

use futures::{Stream, FutureExt};

use crate::SectionError;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

pub type Ack = Pin<Box<dyn Future<Output = ()> + Send>>;
pub type Next<'a> = Pin<Box<dyn Future<Output = Result<Option<Chunk>, SectionError>> + Send>>;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
#[non_exhaustive]
pub enum DataType {
    Null,
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Str,
    Bin,
    Time,
    TimeTz,
    Date,
    TimeStamp,
    TimeStampTz,
    Decimal,
    Uuid,
    RawJson,
    RawJsonB,
    Any, // any of above
}

impl Into<i8> for DataType {
    fn into(self) -> i8 {
        match self {
            Self::Null => 0,
            Self::Bool => 1,
            Self::I8 => 2,
            Self::I16 => 3,
            Self::I32 => 4,
            Self::I64 => 5,
            Self::U8 => 6,
            Self::U16 => 7,
            Self::U32 => 8,
            Self::U64 => 9,
            Self::F32 => 10,
            Self::F64 => 11,
            Self::Str => 12,
            Self::Bin => 13,
            Self::Time => 14,
            Self::TimeTz => 15,
            Self::Date => 16,
            Self::TimeStamp => 17,
            Self::TimeStampTz => 18,
            Self::Decimal => 19,
            Self::Uuid => 20,
            Self::RawJson => 21,
            Self::RawJsonB => 22,
            Self::Any => 127,
        }
    }
}

impl From<i8> for DataType {
    fn from(value: i8) -> Self {
        match value {
            0 => Self::Null,
            1 => Self::Bool,
            2 => Self::I8,
            3 => Self::I16,
            4 => Self::I32,
            5 => Self::I64,
            6 => Self::U8,
            7 => Self::U16,
            8 => Self::U32,
            9 => Self::U64,
            10 => Self::F32,
            11 => Self::F64,
            12 => Self::Str,
            13 => Self::Bin,
            14 => Self::Time,
            15 => Self::TimeTz,
            16 => Self::Date,
            17 => Self::TimeStamp,
            18 => Self::TimeStampTz,
            19 => Self::Decimal,
            20 => Self::Uuid,
            21 => Self::RawJson,
            22 => Self::RawJsonB,
            127 => Self::Any,
            value => panic!("unexpected value: {}", value),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
#[non_exhaustive]
pub enum Value {
    Null,
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
    Str(Box<str>),
    Bin(Box<[u8]>),
    Time(crate::time::Time),
    TimeTz(Box<str>),
    Date(crate::time::Date),
    TimeStamp(crate::time::PrimitiveDateTime),
    TimeStampTz(crate::time::OffsetDateTime),
    Decimal(crate::decimal::Decimal),
    Uuid(crate::uuid::Uuid),
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::Str(value.into())
    }
}

impl From<Vec<u8>> for Value {
    fn from(value: Vec<u8>) -> Self {
        Value::Bin(value.into())
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
#[non_exhaustive]
pub enum ValueView<'a> {
    Null,
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
    Str(&'a str),
    Bin(&'a [u8]),
    Time(&'a time::Time),
    TimeTz(&'a str),
    Date(&'a time::Date),
    TimeStamp(&'a time::PrimitiveDateTime),
    TimeStampTz(&'a time::OffsetDateTime),
    Decimal(&'a rust_decimal::Decimal),
    Uuid(&'a uuid::Uuid),
}

impl ValueView<'_> {
    pub fn data_type(&self) -> DataType {
        match self {
            Self::Null => DataType::Null,
            Self::Bool(_) => DataType::Bool,
            Self::I8(_) => DataType::I8,
            Self::I16(_) => DataType::I16,
            Self::I32(_) => DataType::I32,
            Self::I64(_) => DataType::I64,
            Self::U8(_) => DataType::U8,
            Self::U16(_) => DataType::U16,
            Self::U32(_) => DataType::U32,
            Self::U64(_) => DataType::U64,
            Self::F32(_) => DataType::F32,
            Self::F64(_) => DataType::F64,
            Self::Str(_) => DataType::Str,
            Self::Bin(_) => DataType::Bin,
            Self::Time(_) => DataType::Time,
            Self::TimeTz(_) => DataType::TimeTz,
            Self::Date(_) => DataType::Date,
            Self::TimeStamp(_) => DataType::TimeStamp,
            Self::TimeStampTz(_) => DataType::TimeStampTz,
            Self::Decimal(_) => DataType::Decimal,
            Self::Uuid(_) => DataType::Uuid,
        }
    }
}

impl<'a> PartialEq<Value> for ValueView<'a> {
    fn eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Self::Null, Value::Null) => true,
            (&Self::Bool(l), &Value::Bool(r)) => l == r,
            (&Self::I8(l), &Value::I8(r)) => l == r,
            (&Self::I16(l), &Value::I16(r)) => l == r,
            (&Self::I32(l), &Value::I32(r)) => l == r,
            (&Self::I64(l), &Value::I64(r)) => l == r,
            (&Self::U8(l), &Value::U8(r)) => l == r,
            (&Self::U16(l), &Value::U16(r)) => l == r,
            (&Self::U32(l), &Value::U32(r)) => l == r,
            (&Self::U64(l), &Value::U64(r)) => l == r,
            (&Self::F32(l), &Value::F32(r)) => l == r,
            (&Self::F64(l), &Value::F64(r)) => l == r,
            (&Self::Str(l), Value::Str(r)) => l == r.as_ref(),
            (&Self::Bin(l), Value::Bin(r)) => l == r.as_ref(),
            (&Self::Time(l), Value::Time(r)) => l == r,
            (&Self::TimeTz(l), Value::TimeTz(r)) => l == r.as_ref(),
            (&Self::Date(l), Value::Date(r)) => l == r,
            (&Self::TimeStamp(l), Value::TimeStamp(r)) => l == r,
            (&Self::TimeStampTz(l), Value::TimeStampTz(r)) => l == r,
            (&Self::Decimal(l), Value::Decimal(r)) => l == r,
            (&Self::Uuid(l), Value::Uuid(r)) => l == r,
            _ => false,
        }
    }
}

impl<'a> From<&'a Value> for ValueView<'a> {
    fn from(value: &'a Value) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Bool(v) => Self::Bool(*v),
            Value::I8(v) => Self::I8(*v),
            Value::I16(v) => Self::I16(*v),
            Value::I32(v) => Self::I32(*v),
            Value::I64(v) => Self::I64(*v),
            Value::U8(v) => Self::U8(*v),
            Value::U16(v) => Self::U16(*v),
            Value::U32(v) => Self::U32(*v),
            Value::U64(v) => Self::U64(*v),
            Value::F32(v) => Self::F32(*v),
            Value::F64(v) => Self::F64(*v),
            Value::Str(v) => Self::Str(v),
            Value::Bin(v) => Self::Bin(v),
            Value::Time(v) => Self::Time(v),
            Value::TimeTz(v) => Self::TimeTz(v),
            Value::Date(v) => Self::Date(v),
            Value::TimeStamp(v) => Self::TimeStamp(v),
            Value::TimeStampTz(v) => Self::TimeStampTz(v),
            Value::Decimal(v) => Self::Decimal(v),
            Value::Uuid(v) => Self::Uuid(v),
        }
    }
}

impl<'a> From<&'a String> for ValueView<'a> {
    fn from(value: &'a String) -> Self {
        ValueView::Str(value)
    }
}

impl<'a> From<&'a str> for ValueView<'a> {
    fn from(value: &'a str) -> Self {
        ValueView::Str(value)
    }
}

pub trait Message: Send + std::fmt::Debug {
    fn origin(&self) -> &str;

    fn next(&mut self) -> Next<'_>;

    fn ack(&mut self) -> Ack;
}

pub struct MessageStream<'a> {
    inner: Box<dyn Message>,
    future: Option<Next<'a>>,
    _marker: PhantomData<&'a ()>,
}

unsafe impl Sync for MessageStream<'_>{}

impl<'a> Stream for MessageStream<'a> {
    type Item = Result<Chunk, SectionError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.as_mut();
        let mut future = match this.future.take() {
            None => this.inner.next(),
            Some(f) => f,
        };
        match future.poll_unpin(cx) {
            Poll::Pending => {
                this.future = Some(future);
                Poll::Pending
            }
            Poll::Ready(Ok(None)) => {
                Poll::Ready(None)
            },
            Poll::Ready(Ok(Some(chunk))) => Poll::Ready(Some(Ok(chunk))),
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
        }
    }
}

impl From<Box<dyn Message>> for MessageStream<'_> {
    fn from(inner: Box<dyn Message>) -> Self {
        Self { inner, future: None, _marker: PhantomData } 
    }
}

#[derive(Debug)]
pub enum Chunk {
    Byte(Vec<u8>),
    DataFrame(Box<dyn DataFrame>),
}

pub trait DataFrame: std::fmt::Debug + Send + 'static {
    fn columns(&self) -> Vec<Column<'_>>;
}

pub struct Column<'a> {
    name: &'a str,
    data_type: DataType,
    iter: Box<dyn Iterator<Item = ValueView<'a>> + 'a + Send>,
}

impl<'a> std::fmt::Debug for Column<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Column")
            .field("name", &self.name)
            .field("data_type", &self.data_type)
            .finish()
    }
}

impl<'a> Column<'a> {
    pub fn new(
        name: &'a str,
        data_type: DataType,
        iter: Box<dyn Iterator<Item = ValueView<'a>> + 'a + Send>,
    ) -> Self {
        Self {
            name,
            data_type,
            iter,
        }
    }

    pub fn name(&self) -> &str {
        self.name
    }

    pub fn data_type(&self) -> DataType {
        self.data_type
    }
}

impl<'a> Iterator for Column<'a> {
    type Item = ValueView<'a>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use futures::TryStream;

    use super::*;

    #[test]
    fn size_of_value() {
        assert!(24 >= std::mem::size_of::<Value>());
    }

    #[test]
    fn size_of_value_view() {
        assert!(24 >= std::mem::size_of::<ValueView>());
    }

    #[test]
    fn size_datatype_converts() {
        let mut set = HashSet::new();
        let len = 23;
        for x in (0..len).chain(std::iter::once(127)) {
            let x = x as i8;
            let dt: DataType = x.into();
            set.insert(dt);
            assert_eq!(<DataType as Into<i8>>::into(dt), x);
        };
        assert_eq!(set.len(), len + 1);
    }

    #[tokio::test]
    async fn test_message_stream() {
        #[derive(Debug)]
        struct Test {
            inner: Vec<i32>,
        }

        impl DataFrame for Test {
            fn columns(&self) -> Vec<Column<'_>> {
                vec![
                    Column::new("inner", DataType::I32, Box::new(self.inner.iter().copied().map(ValueView::I32)))
                ]
            }

        }

        #[derive(Debug)]
        struct TestMsg {
            inner: Option<Test>
        }
        
        impl Message for TestMsg {
            fn ack(&mut self) -> Ack {
                Box::pin(async {})
            }

            fn origin(&self) -> &str {
                "test"
            }

            fn next(&mut self) -> Next<'_> {
                let v = self.inner.take().map(|v| Chunk::DataFrame(Box::new(v)));
                Box::pin(async move { Ok(v) })
            }
        }

        let msg = TestMsg {
            inner: Some(Test { inner: vec![1, 2, 3] })
        };

        let msg_stream: MessageStream = (Box::new(msg) as Box<dyn Message>).into();

        fn try_stream<T: TryStream>(_: &T){}
        try_stream(&msg_stream);

        fn send_sync<T: Send + Sync + 'static>(_: T){}
        send_sync(msg_stream);
    }
}
