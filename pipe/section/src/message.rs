//! Section messaging

use rust_decimal::prelude::FromPrimitive;

use crate::SectionError;
use std::fmt;
use std::future::Future;
use std::pin::Pin;

pub type Ack = Pin<Box<dyn Future<Output = ()> + Send>>;
pub type Next<'a> = Pin<Box<dyn Future<Output = Result<Option<Chunk>, SectionError>> + 'a + Send>>;

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
    Time(TimeUnit),
    Date(TimeUnit),
    TimeStamp(TimeUnit),
    TimeStampUTC(TimeUnit),
    Decimal,
    Uuid,
    RawJson,
    Any, // any of above
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<DataType> for i8 {
    fn from(value: DataType) -> i8 {
        match value {
            DataType::Null => 0,
            DataType::Bool => 1,
            DataType::I8 => 2,
            DataType::I16 => 3,
            DataType::I32 => 4,
            DataType::I64 => 5,
            DataType::U8 => 6,
            DataType::U16 => 7,
            DataType::U32 => 8,
            DataType::U64 => 9,
            DataType::F32 => 10,
            DataType::F64 => 11,
            DataType::Str => 12,
            DataType::Bin => 13,
            DataType::Time(TimeUnit::Second) => 14,
            DataType::Time(TimeUnit::Millisecond) => 15,
            DataType::Time(TimeUnit::Microsecond) => 16,
            DataType::Time(TimeUnit::Nanosecond) => 17,
            DataType::Date(TimeUnit::Second) => 18,
            DataType::Date(TimeUnit::Millisecond) => 19,
            DataType::Date(TimeUnit::Microsecond) => 20,
            DataType::Date(TimeUnit::Nanosecond) => 21,
            DataType::TimeStamp(TimeUnit::Second) => 22,
            DataType::TimeStamp(TimeUnit::Millisecond) => 23,
            DataType::TimeStamp(TimeUnit::Microsecond) => 24,
            DataType::TimeStamp(TimeUnit::Nanosecond) => 25,
            DataType::TimeStampUTC(TimeUnit::Second) => 26,
            DataType::TimeStampUTC(TimeUnit::Millisecond) => 27,
            DataType::TimeStampUTC(TimeUnit::Microsecond) => 28,
            DataType::TimeStampUTC(TimeUnit::Nanosecond) => 29,
            DataType::Decimal => 30,
            DataType::Uuid => 31,
            DataType::RawJson => 32,
            DataType::Any => 127,
        }
    }
}

impl From<i8> for DataType {
    fn from(value: i8) -> Self {
        match value {
            0 => DataType::Null,
            1 => DataType::Bool,
            2 => DataType::I8,
            3 => DataType::I16,
            4 => DataType::I32,
            5 => DataType::I64,
            6 => DataType::U8,
            7 => DataType::U16,
            8 => DataType::U32,
            9 => DataType::U64,
            10 => DataType::F32,
            11 => DataType::F64,
            12 => DataType::Str,
            13 => DataType::Bin,
            14 => DataType::Time(TimeUnit::Second),
            15 => DataType::Time(TimeUnit::Millisecond),
            16 => DataType::Time(TimeUnit::Microsecond),
            17 => DataType::Time(TimeUnit::Nanosecond),
            18 => DataType::Date(TimeUnit::Second),
            19 => DataType::Date(TimeUnit::Millisecond),
            20 => DataType::Date(TimeUnit::Microsecond),
            21 => DataType::Date(TimeUnit::Nanosecond),
            22 => DataType::TimeStamp(TimeUnit::Second),
            23 => DataType::TimeStamp(TimeUnit::Millisecond),
            24 => DataType::TimeStamp(TimeUnit::Microsecond),
            25 => DataType::TimeStamp(TimeUnit::Nanosecond),
            26 => DataType::TimeStampUTC(TimeUnit::Second),
            27 => DataType::TimeStampUTC(TimeUnit::Millisecond),
            28 => DataType::TimeStampUTC(TimeUnit::Microsecond),
            29 => DataType::TimeStampUTC(TimeUnit::Nanosecond),
            30 => DataType::Decimal,
            31 => DataType::Uuid,
            32 => DataType::RawJson,
            127 => DataType::Any,
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
    Time(TimeUnit, i64),
    Date(TimeUnit, i64),
    TimeStamp(TimeUnit, i64),
    TimeStampUTC(TimeUnit, i64),
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

impl<'a> From<&'a ValueView<'_>> for Value {
    fn from(value: &'a ValueView) -> Self {
        match value {
            ValueView::Null => Self::Null,
            ValueView::Bool(v) => Self::Bool(*v),
            ValueView::I8(v) => Self::I8(*v),
            ValueView::I16(v) => Self::I16(*v),
            ValueView::I32(v) => Self::I32(*v),
            ValueView::I64(v) => Self::I64(*v),
            ValueView::U8(v) => Self::U8(*v),
            ValueView::U16(v) => Self::U16(*v),
            ValueView::U32(v) => Self::U32(*v),
            ValueView::U64(v) => Self::U64(*v),
            ValueView::F32(v) => Self::F32(*v),
            ValueView::F64(v) => Self::F64(*v),
            ValueView::Str(v) => Self::Str(v.to_string().into_boxed_str()),
            ValueView::Bin(v) => Self::Bin(v.to_vec().into_boxed_slice()),
            ValueView::Time(tu, v) => Self::Time(*tu, *v),
            ValueView::Date(tu, v) => Self::Date(*tu, *v),
            ValueView::TimeStamp(tu, v) => Self::TimeStamp(*tu, *v),
            ValueView::TimeStampUTC(tu, v) => Self::TimeStampUTC(*tu, *v),
            ValueView::Decimal(v) => Self::Decimal(*v),
            ValueView::Uuid(v) => Self::Uuid(**v),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeCastError {
    msg: String,
}

impl fmt::Display for TypeCastError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl Value {
    pub fn into_u16(self) -> Result<Value, TypeCastError> {
        match self {
            Value::U8(v) => Ok(Value::U16(v.into())),
            Value::U16(v) => Ok(Value::U16(v)),
            t => Err(TypeCastError {
                msg: format!("could not convert {:?} to Value::U8", t),
            }),
        }
    }

    pub fn into_u32(self) -> Result<Value, TypeCastError> {
        match self {
            Value::U8(v) => Ok(Value::U32(v.into())),
            Value::U16(v) => Ok(Value::U32(v.into())),
            Value::U32(v) => Ok(Value::U32(v)),
            t => Err(TypeCastError {
                msg: format!("could not convert {:?} to Value::U32", t),
            }),
        }
    }

    pub fn into_u64(self) -> Result<Value, TypeCastError> {
        match self {
            Value::U8(v) => Ok(Value::U64(v.into())),
            Value::U16(v) => Ok(Value::U64(v.into())),
            Value::U32(v) => Ok(Value::U64(v.into())),
            Value::U64(v) => Ok(Value::U64(v)),
            t => Err(TypeCastError {
                msg: format!("could not convert {:?} to Value::U64", t),
            }),
        }
    }

    pub fn into_f32(self) -> Result<Value, TypeCastError> {
        match self {
            Value::U8(v) => Ok(Value::F32(v.into())),
            Value::U16(v) => Ok(Value::F32(v.into())),
            Value::I8(v) => Ok(Value::F32(v.into())),
            Value::I16(v) => Ok(Value::F32(v.into())),
            t => Err(TypeCastError {
                msg: format!("could not convert {:?} to Value::F32", t),
            }),
        }
    }

    pub fn into_f64(self) -> Result<Value, TypeCastError> {
        match self {
            Value::U8(v) => Ok(Value::F64(v.into())),
            Value::U16(v) => Ok(Value::F64(v.into())),
            Value::U32(v) => Ok(Value::F64(v.into())),
            Value::I8(v) => Ok(Value::F64(v.into())),
            Value::I16(v) => Ok(Value::F64(v.into())),
            Value::I32(v) => Ok(Value::F64(v.into())),
            Value::F32(v) => Ok(Value::F64(v.into())),
            t => Err(TypeCastError {
                msg: format!("could not convert {:?} to Value::F64", t),
            }),
        }
    }

    pub fn into_i16(self) -> Result<Value, TypeCastError> {
        match self {
            Value::U8(v) => Ok(Value::I16(v.into())),
            Value::I8(v) => Ok(Value::I16(v.into())),
            t => Err(TypeCastError {
                msg: format!("could not convert {:?} to Value::I16", t),
            }),
        }
    }

    pub fn into_i32(self) -> Result<Value, TypeCastError> {
        match self {
            Value::I8(v) => Ok(Value::I32(v.into())),
            Value::I16(v) => Ok(Value::I32(v.into())),
            Value::U8(v) => Ok(Value::I32(v.into())),
            Value::U16(v) => Ok(Value::I32(v.into())),
            t => Err(TypeCastError {
                msg: format!("could not convert {:?} to Value::I32", t),
            }),
        }
    }

    pub fn into_i64(self) -> Result<Value, TypeCastError> {
        match self {
            Value::I8(v) => Ok(Value::I64(v.into())),
            Value::I16(v) => Ok(Value::I64(v.into())),
            Value::I32(v) => Ok(Value::I64(v.into())),
            Value::U8(v) => Ok(Value::I64(v.into())),
            Value::U16(v) => Ok(Value::I64(v.into())),
            Value::U32(v) => Ok(Value::I64(v.into())),
            t => Err(TypeCastError {
                msg: format!("could not convert {:?} to Value::I32", t),
            }),
        }
    }

    pub fn into_decimal(self) -> Result<Value, TypeCastError> {
        match self {
            Value::I8(v) => Ok(Value::Decimal(v.into())),
            Value::I16(v) => Ok(Value::Decimal(v.into())),
            Value::I32(v) => Ok(Value::Decimal(v.into())),
            Value::I64(v) => Ok(Value::Decimal(v.into())),
            Value::U8(v) => Ok(Value::Decimal(v.into())),
            Value::U16(v) => Ok(Value::Decimal(v.into())),
            Value::U32(v) => Ok(Value::Decimal(v.into())),
            Value::U64(v) => Ok(Value::Decimal(v.into())),
            // TODO: figure out how to cast these
            Value::F32(v) => Ok(Value::Decimal(
                crate::decimal::Decimal::from_f32(v).ok_or_else(|| TypeCastError {
                    msg: format!("could not convert {:?} to Decimal", v),
                })?,
            )),
            Value::F64(v) => Ok(Value::Decimal(
                crate::decimal::Decimal::from_f64(v).ok_or_else(|| TypeCastError {
                    msg: format!("could not convert {:?} to Decimal", v),
                })?,
            )),
            t => Err(TypeCastError {
                msg: format!("could not convert {:?} to Value::I32", t),
            }),
        }
    }

    pub fn into_str(self) -> Result<Value, TypeCastError> {
        Ok(match self {
            Value::Null => Value::Str("null".into()),
            Value::Bool(v) => Value::Str(format!("{}", v).into()),
            Value::I8(v) => Value::Str(format!("{}", v).into()),
            Value::I16(v) => Value::Str(format!("{}", v).into()),
            Value::I32(v) => Value::Str(format!("{}", v).into()),
            Value::I64(v) => Value::Str(format!("{}", v).into()),
            Value::U8(v) => Value::Str(format!("{}", v).into()),
            Value::U16(v) => Value::Str(format!("{}", v).into()),
            Value::U32(v) => Value::Str(format!("{}", v).into()),
            Value::U64(v) => Value::Str(format!("{}", v).into()),
            Value::F32(v) => Value::Str(format!("{}", v).into()),
            Value::F64(v) => Value::Str(format!("{}", v).into()),
            Value::Str(v) => Value::Str(format!("{}", v).into()),
            Value::Bin(_) => Value::Str("<binary blob>".into()),
            Value::Time(u, v) => Value::Str(format!("{:?}({})", u, v).into()),
            Value::Date(u, v) => Value::Str(format!("{:?}({})", u, v).into()),
            Value::TimeStamp(u, v) => Value::Str(format!("{:?}({})", u, v).into()),
            Value::TimeStampUTC(u, v) => Value::Str(format!("{:?}({})", u, v).into()),
            Value::Decimal(v) => Value::Str(format!("{}", v).into()),
            Value::Uuid(v) => Value::Str(format!("{}", v).into()),
        })
        //Ok(Value::Str(format!("{}", self).into()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimeUnit {
    Second,
    Millisecond,
    Microsecond,
    Nanosecond,
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
    Time(TimeUnit, i64),
    Date(TimeUnit, i64),
    TimeStamp(TimeUnit, i64),
    TimeStampUTC(TimeUnit, i64),
    Decimal(rust_decimal::Decimal),
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
            Self::Time(tu, _) => DataType::Time(*tu),
            Self::Date(tu, _) => DataType::Date(*tu),
            Self::TimeStamp(tu, _) => DataType::TimeStamp(*tu),
            Self::TimeStampUTC(tu, _) => DataType::TimeStampUTC(*tu),
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
            (&Self::Time(ltu, l), &Value::Time(rtu, r)) => l == r && ltu == rtu,
            (&Self::Date(ltu, l), &Value::Date(rtu, r)) => l == r && ltu == rtu,
            (&Self::TimeStamp(ltu, l), &Value::TimeStamp(rtu, r)) => l == r && ltu == rtu,
            (&Self::TimeStampUTC(ltu, l), &Value::TimeStampUTC(rtu, r)) => l == r && ltu == rtu,
            (&Self::Decimal(l), Value::Decimal(r)) => l == *r,
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
            Value::Time(tu, v) => Self::Time(*tu, *v),
            Value::Date(tu, v) => Self::Date(*tu, *v),
            Value::TimeStamp(tu, v) => Self::TimeStamp(*tu, *v),
            Value::TimeStampUTC(tu, v) => Self::TimeStampUTC(*tu, *v),
            Value::Decimal(v) => Self::Decimal(*v),
            Value::Uuid(v) => Self::Uuid(v),
        }
    }
}

impl<'a> From<&'a u32> for ValueView<'a> {
    fn from(value: &'a u32) -> Self {
        ValueView::U32(*value)
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
    use std::collections::HashSet;

    use rust_decimal::Decimal;

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
        let len = 33;
        for x in (0..len).chain(std::iter::once(127)) {
            let x = x as i8;
            let dt: DataType = x.into();
            set.insert(dt);
            assert_eq!(<DataType as Into<i8>>::into(dt), x);
        }
        assert_eq!(set.len(), len + 1);
    }

    #[rustfmt::skip]
    #[test]
    fn upcast_value() {
        // we don't have into_u8, since it's a minimum size

        // into U16
        assert_eq!(
            Value::U16(2), 
            Value::U16(2).into_u16().unwrap()
        );
        assert_eq!(
            Value::U16(2), 
            Value::U8(2).into_u16().unwrap()
        );
        assert!(
            Value::U32(2).into_u16().is_err()
        );

        // into U32
        assert_eq!(
            Value::U32(2), 
            Value::U32(2).into_u32().unwrap()
        );
        assert_eq!(
            Value::U32(2), 
            Value::U16(2).into_u32().unwrap()
        );
        assert_eq!(
            Value::U32(2), 
            Value::U8(2).into_u32().unwrap()
        );
        assert!(
            Value::U32(2).into_u16().is_err()
        );

        // into U64
        assert_eq!(
            Value::U64(2), 
            Value::U64(2).into_u64().unwrap()
        );
        assert_eq!(
            Value::U64(2), 
            Value::U32(2).into_u64().unwrap()
        );
        assert_eq!(
            Value::U64(2), 
            Value::U16(2).into_u64().unwrap()
        );
        assert_eq!(
            Value::U64(2), 
            Value::U8(2).into_u64().unwrap()
        );
        assert!(
            Value::I64(2).into_u64().is_err()
        );

        // into F32
        assert_eq!(
            Value::F32(2.), 
            Value::U8(2).into_f32().unwrap()
        );
        assert_eq!(
            Value::F32(2.), 
            Value::U16(2).into_f32().unwrap()
        );
        assert_eq!(
            Value::F32(2.), 
            Value::I8(2).into_f32().unwrap()
        );
        assert_eq!(
            Value::F32(2.), 
            Value::I16(2).into_f32().unwrap()
        );
        assert!(
            Value::I64(2).into_f32().is_err()
        );

        // into F64
        assert_eq!(
            Value::F64(2.), 
            Value::U8(2).into_f64().unwrap()
        );
        assert_eq!(
            Value::F64(2.), 
            Value::U16(2).into_f64().unwrap()
        );
        assert_eq!(
            Value::F64(2.), 
            Value::U32(2).into_f64().unwrap()
        );
        assert_eq!(
            Value::F64(2.), 
            Value::I8(2).into_f64().unwrap()
        );
        assert_eq!(
            Value::F64(2.), 
            Value::I16(2).into_f64().unwrap()
        );
        assert_eq!(
            Value::F64(2.), 
            Value::I32(2).into_f64().unwrap()
        );
        assert_eq!(
            Value::F64(2.), 
            Value::F32(2.).into_f64().unwrap()
        );
        assert!(
            Value::I64(2).into_f64().is_err()
        );

        // into I16
        assert_eq!(
            Value::I16(2), 
            Value::U8(2).into_i16().unwrap()
        );
        assert_eq!(
            Value::I16(2), 
            Value::I8(2).into_i16().unwrap()
        );
        assert!(
            Value::I64(2).into_f64().is_err()
        );

        // into F32
        assert_eq!(
            Value::F32(2.), 
            Value::U8(2).into_f32().unwrap()
        );
        assert_eq!(
            Value::F32(2.), 
            Value::U16(2).into_f32().unwrap()
        );
        assert_eq!(
            Value::F32(2.), 
            Value::I8(2).into_f32().unwrap()
        );
        assert_eq!(
            Value::F32(2.), 
            Value::I16(2).into_f32().unwrap()
        );
        assert!(
            Value::I64(2).into_f32().is_err()
        );

        // into F64
        assert_eq!(
            Value::F64(2.), 
            Value::U8(2).into_f64().unwrap()
        );
        assert_eq!(
            Value::F64(2.), 
            Value::U16(2).into_f64().unwrap()
        );
        assert_eq!(
            Value::F64(2.), 
            Value::U32(2).into_f64().unwrap()
        );
        assert_eq!(
            Value::F64(2.), 
            Value::I8(2).into_f64().unwrap()
        );
        assert_eq!(
            Value::F64(2.), 
            Value::I16(2).into_f64().unwrap()
        );
        assert_eq!(
            Value::F64(2.), 
            Value::I32(2).into_f64().unwrap()
        );
        assert!(
            Value::I64(2).into_f64().is_err()
        );

        // into decimal
        assert_eq!(
            Value::Decimal(Decimal::new(2, 0)), 
            Value::I8(2).into_decimal().unwrap()
        );
        assert_eq!(
            Value::Decimal(Decimal::new(2, 0)), 
            Value::I16(2).into_decimal().unwrap()
        );
        assert_eq!(
            Value::Decimal(Decimal::new(2, 0)), 
            Value::I32(2).into_decimal().unwrap()
        );
        assert_eq!(
            Value::Decimal(Decimal::new(2, 0)), 
            Value::I64(2).into_decimal().unwrap()
        );
        assert_eq!(
            Value::Decimal(Decimal::new(2, 0)), 
            Value::U8(2).into_decimal().unwrap()
        );
        assert_eq!(
            Value::Decimal(Decimal::new(2, 0)), 
            Value::U16(2).into_decimal().unwrap()
        );
        assert_eq!(
            Value::Decimal(Decimal::new(2, 0)), 
            Value::U32(2).into_decimal().unwrap()
        );
        assert_eq!(
            Value::Decimal(Decimal::new(2, 0)), 
            Value::U64(2).into_decimal().unwrap()
        );
        assert_eq!(
            Value::Decimal(Decimal::new(314159, 5)), 
            Value::F32(3.14159).into_decimal().unwrap()
        );
        assert_eq!(
            Value::Decimal(Decimal::new(314159265, 8)), 
            Value::F64(3.14159265).into_decimal().unwrap()
        );

        // into str
        assert_eq!(
            Value::Str("null".into()),
            Value::Null.into_str().unwrap()
        );
        assert_eq!(
            Value::Str("true".into()),
            Value::Bool(true).into_str().unwrap()
        );
        assert_eq!(
            Value::Str("2".into()),
            Value::I8(2).into_str().unwrap()
        );
        assert_eq!(
            Value::Str("2".into()),
            Value::I16(2).into_str().unwrap()
        );
        assert_eq!(
            Value::Str("2".into()),
            Value::I32(2).into_str().unwrap()
        );
        assert_eq!(
            Value::Str("2".into()),
            Value::I64(2).into_str().unwrap()
        );
        assert_eq!(
            Value::Str("2".into()),
            Value::U8(2).into_str().unwrap()
        );
        assert_eq!(
            Value::Str("2".into()),
            Value::U16(2).into_str().unwrap()
        );
        assert_eq!(
            Value::Str("2".into()),
            Value::U32(2).into_str().unwrap()
        );
        assert_eq!(
            Value::Str("2".into()),
            Value::U64(2).into_str().unwrap()
        );
        assert_eq!(
            Value::Str("3.14".into()),
            Value::F32(3.14).into_str().unwrap()
        );
        assert_eq!(
            Value::Str("3.14".into()),
            Value::F64(3.14).into_str().unwrap()
        );
        assert_eq!(
            Value::Str("2".into()),
            Value::Str("2".into()).into_str().unwrap()
        );
        assert_eq!(
            Value::Str("<binary blob>".into()),
            Value::Bin(vec![1].into()).into_str().unwrap()
        );
        assert_eq!(
            Value::Str("Millisecond(123)".into()),
            Value::Time(TimeUnit::Millisecond, 123).into_str().unwrap()
        );
        assert_eq!(
            Value::Str("Millisecond(123)".into()),
            Value::Date(TimeUnit::Millisecond, 123).into_str().unwrap()
        );
        assert_eq!(
            Value::Str("Millisecond(123)".into()),
            Value::TimeStamp(TimeUnit::Millisecond, 123).into_str().unwrap()
        );
        assert_eq!(
            Value::Str("Millisecond(123)".into()),
            Value::TimeStampUTC(TimeUnit::Millisecond, 123).into_str().unwrap()
        );
        assert_eq!(
            Value::Str("3.14159".into()),
            Value::Decimal(Decimal::new(314159, 5)).into_str().unwrap()
        );
        assert_eq!(
            Value::Str("a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8".into()),
            Value::Uuid(crate::uuid::Uuid::parse_str("a1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8").unwrap()).into_str().unwrap()
        );
    }
}
