//! Section messaging

use crate::SectionError;
use std::future::Future;
use std::pin::Pin;

pub type Ack = Pin<Box<dyn Future<Output = ()> + Send>>;
pub type Next<'a> = Pin<Box<dyn Future<Output = Result<Option<Chunk>, SectionError>> + Send>>;

#[derive(Debug, PartialEq, Clone, Copy)]
#[non_exhaustive]
pub enum DataType {
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
    Any,
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
    Date(&'a time::Date),
    TimeStamp(&'a time::PrimitiveDateTime),
    TimeStampTz(&'a time::OffsetDateTime),
    Decimal(&'a rust_decimal::Decimal),
    Uuid(&'a uuid::Uuid),
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
    use super::*;

    #[test]
    fn size_of_value() {
        assert!(24 >= std::mem::size_of::<Value>());
    }

    #[test]
    fn size_of_value_view() {
        assert!(24 >= std::mem::size_of::<ValueView>());
    }
}
