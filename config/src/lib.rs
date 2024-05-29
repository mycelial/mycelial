pub use config_derive::Config;

pub mod prelude {
    pub use super::Config as _;
    pub use super::{Field, FieldType, FieldValue, Metadata, SectionIO};
    pub use config_derive::Config;
}
mod de;
mod ser;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SectionIO {
    None,
    Bin,
    DataFrame,
}

pub type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub trait Config: std::fmt::Debug {
    fn name(&self) -> &str;

    fn input(&self) -> SectionIO;

    fn output(&self) -> SectionIO;

    fn fields(&self) -> Vec<Field>;

    fn set_field_value(&mut self, name: &str, value: &str) -> Result<(), StdError>;

    fn validate_field(&self, field_name: &str, value: &str) -> Result<(), StdError> {
        let field = self
            .fields()
            .into_iter()
            .filter(|field| field.name == field_name)
            .collect::<Vec<_>>();
        match field.as_slice() {
            [field] => {
                let _: FieldValue = (&field.ty, value).try_into()?;
                Ok(())
            }
            [] => Err("no such field")?,
            _ => Err("multiple fields with such name")?, // should not be possible in current implementation
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
}

impl FieldType {
    pub fn is_bool(&self) -> bool {
        self == &Self::Bool
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct Metadata {
    pub is_password: bool,
    pub is_text_area: bool,
}

#[derive(Debug, PartialEq)]
pub struct Field<'a> {
    pub name: &'a str,
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
    Vec(Vec<FieldValue<'a>>),
}

impl std::fmt::Display for FieldValue<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldValue::I8(v) => write!(f, "{v}"),
            FieldValue::I16(v) => write!(f, "{v}"),
            FieldValue::I32(v) => write!(f, "{v}"),
            FieldValue::I64(v) => write!(f, "{v}"),
            FieldValue::U8(v) => write!(f, "{v}"),
            FieldValue::U16(v) => write!(f, "{v}"),
            FieldValue::U32(v) => write!(f, "{v}"),
            FieldValue::U64(v) => write!(f, "{v}"),
            FieldValue::String(v) => write!(f, "{v}"),
            FieldValue::Bool(v) => write!(f, "{v}"),
            FieldValue::Vec(_v) => unimplemented!("display for vec"),
        }
    }
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
where
    FieldValue<'a>: From<&'a T>,
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
    };
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

impl<'a> TryFrom<(&'a FieldType, &'a str)> for FieldValue<'a> {
    type Error = StdError;

    fn try_from((ty, value): (&'a FieldType, &'a str)) -> Result<Self, Self::Error> {
        let field_value = match ty {
            FieldType::I8 => FieldValue::I8(value.parse()?),
            FieldType::I16 => FieldValue::I16(value.parse()?),
            FieldType::I32 => FieldValue::I32(value.parse()?),
            FieldType::I64 => FieldValue::I64(value.parse()?),
            FieldType::U8 => FieldValue::U8(value.parse()?),
            FieldType::U16 => FieldValue::U16(value.parse()?),
            FieldType::U32 => FieldValue::U32(value.parse()?),
            FieldType::U64 => FieldValue::U64(value.parse()?),
            FieldType::String => FieldValue::String(value),
            FieldType::Bool => FieldValue::Bool(value.parse()?),
        };
        Ok(field_value)
    }
}

macro_rules! try_into_field_value_impl {
    ($ty:ty, $arm:tt, $var:tt, $($conv:tt)*) => {
        impl TryInto<$ty> for &FieldValue<'_> {
            type Error = StdError;

            fn try_into(self) -> Result<$ty, Self::Error> {
                match self {
                    FieldValue::$arm($var) => Ok($($conv)*),
                    _ => Err(format!("Can't convert {:?} into {}", self, stringify!($arm)))?
                }
            }
        }
    }
}

try_into_field_value_impl!(u8, U8, v, *v);
try_into_field_value_impl!(u16, U16, v, *v);
try_into_field_value_impl!(u32, U32, v, *v);
try_into_field_value_impl!(u64, U64, v, *v);
try_into_field_value_impl!(i8, I8, v, *v);
try_into_field_value_impl!(i16, I16, v, *v);
try_into_field_value_impl!(i32, I32, v, *v);
try_into_field_value_impl!(i64, I64, v, *v);
try_into_field_value_impl!(String, String, v, v.to_string());
try_into_field_value_impl!(bool, Bool, v, *v);
