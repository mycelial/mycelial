pub use config_derive::Config;

pub mod prelude {
    pub use super::Config as _;
    pub use super::{Field, FieldType, FieldValue, Metadata, SectionIO};
    pub use config_derive::Config;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SectionIO {
    None,
    Bin,
    DataFrame,
}
pub trait Config {
    fn input(&self) -> SectionIO;
    fn output(&self) -> SectionIO;
    fn fields(&self) -> Vec<Field>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    String,
    Bool,
    Vec(Box<FieldType>),
}

impl FieldType {
    pub fn is_bool(&self) -> bool {
        self == &Self::Bool
    }

    pub fn is_vec(&self) -> bool {
        matches!(self, Self::Vec(_))
    }
}

#[derive(Debug, PartialEq)]
pub struct Metadata {
    pub is_password: bool,
    pub is_text_area: bool,
}

#[derive(Debug, PartialEq)]
pub struct Field<'a> {
    pub name: &'static str,
    pub ty: FieldType,
    pub metadata: Metadata,
    pub value: FieldValue<'a>,
}

#[derive(Debug, PartialEq)]
pub enum FieldValue<'a> {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    String(&'a str),
    Bool(bool),
    Vec(Vec<FieldValue<'a>>)
}

macro_rules! impl_from_ref {
    ($ty:ty, $arm:tt, $($op:tt)+) => {
        impl<'a> From<&'a $ty> for FieldValue<'a> {
            fn from(value: &'a $ty) -> FieldValue<'a> {
                FieldValue::$arm($($op)* value)
            }
        }
    }
}

impl_from_ref!(u8, U8, *);
impl_from_ref!(u16, U16, *);
impl_from_ref!(u32, U32, *);
impl_from_ref!(u64, U64, *);
impl_from_ref!(i8, I8, *);
impl_from_ref!(i16, I16, *);
impl_from_ref!(i32, I32, *);
impl_from_ref!(i64, I64, *);
impl_from_ref!(bool, Bool, *);
impl_from_ref!(String, String, &*);

impl<'a, T> From<&'a Vec<T>> for FieldValue<'a>
    where FieldValue<'a>: From<&'a T>
{
    fn from(value: &'a Vec<T>) -> FieldValue<'a> {
        FieldValue::Vec(value.iter().map(Into::into).collect())
    }
}

macro_rules! impl_from_value {
    ($ty:ty, $arm:tt) => {
        impl<'a> From<$ty> for FieldValue<'a> {
            fn from(value: $ty) -> FieldValue<'a> {
                FieldValue::$arm(value)
            }
        }
    }
}

impl_from_value!(u8, U8);
impl_from_value!(u16, U16);
impl_from_value!(u32, U32);
impl_from_value!(u64, U64);
impl_from_value!(i8, I8);
impl_from_value!(i16, I16);
impl_from_value!(i32, I32);
impl_from_value!(i64, I64);
impl_from_value!(bool, Bool);
impl_from_value!(&'static str, String);