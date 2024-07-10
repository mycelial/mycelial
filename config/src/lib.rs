mod de;
mod ser;

pub use config_derive::Config;
use dyn_clone::DynClone;

pub mod prelude {
    pub use super::de::deserialize_into_config;
    pub use super::Config;
    pub use super::{Field, FieldType, FieldValue, Metadata, SectionIO};
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SectionIO {
    None,
    Bin,
    DataFrame,
}

impl SectionIO {
    pub fn is_none(self) -> bool {
        SectionIO::None == self
    }
}

pub fn clone_config(config: &dyn Config) -> Box<dyn Config> {
    dyn_clone::clone_box(config)
}

pub type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub trait Config: std::fmt::Debug + DynClone + std::any::Any + Send + Sync + 'static {
    fn name(&self) -> &str;

    fn input(&self) -> SectionIO;

    fn output(&self) -> SectionIO;

    fn fields(&self) -> Vec<Field<'_>>;

    fn get_field_value(&self, name: &str) -> Result<FieldValue<'_>, StdError>;

    fn set_field_value(&mut self, name: &str, value: &FieldValue<'_>) -> Result<(), StdError>;

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
    pub is_read_only: bool,
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
}

impl FieldValue<'_> {
    pub fn field_type(&self) -> FieldType {
        match self {
            FieldValue::I8(_) => FieldType::I8,
            FieldValue::I16(_) => FieldType::I16,
            FieldValue::I32(_) => FieldType::I32,
            FieldValue::I64(_) => FieldType::I64,
            FieldValue::U8(_) => FieldType::U8,
            FieldValue::U16(_) => FieldType::U16,
            FieldValue::U32(_) => FieldType::U32,
            FieldValue::U64(_) => FieldType::U64,
            FieldValue::String(_) => FieldType::String,
            FieldValue::Bool(_) => FieldType::Bool,
        }
    }
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
impl_from_value!(&'a str, String);

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
    ($var:ident, $ty:ty, $($arm:tt),* => $expr:tt, $special_case:tt => $special_expr:tt)  => {
        impl TryInto<$ty> for &FieldValue<'_> {
            type Error = StdError;

            fn try_into(self) -> Result<$ty, Self::Error> {
                match self {
                    $(FieldValue::$arm($var) => Ok(try_into_field_value_impl!(@expand_expr, $expr)),)*
                    FieldValue::$special_case($var) => Ok(try_into_field_value_impl!(@expand_expr, $special_expr)),
                    _ => Err(format!("Can't convert {:?} into {}", self, stringify!($ty)))?
                }
            }
        }
    };
    ($var:ident, $ty:ty, $($arm:tt),* => $expr:tt)  => {
        impl TryInto<$ty> for &FieldValue<'_> {
            type Error = StdError;

            fn try_into(self) -> Result<$ty, Self::Error> {
                match self {
                    $(FieldValue::$arm($var) => Ok(try_into_field_value_impl!(@expand_expr, $expr)),)*
                    _ => Err(format!("Can't convert {:?} into {}", self, stringify!($ty)))?
                }
            }
        }
    };
    (@expand_expr, { $($token:tt)+ }) => {
        $($token)*
    }
}

try_into_field_value_impl!(v, u8,
    U8, U16, U32, U64, I8, I16, I32, I64 => { (*v).try_into()? },
    String => { v.parse()? }
);
try_into_field_value_impl!(v, u16,
    U8, U16, U32, U64, I8, I16, I32, I64 => { (*v).try_into()? },
    String => { v.parse()? }
);
try_into_field_value_impl!(v, u32,
    U8, U16, U32, U64, I8, I16, I32, I64 => { (*v).try_into()? },
    String => { v.parse()? }
);
try_into_field_value_impl!(v, u64,
    U8, U16, U32, U64, I8, I16, I32, I64 => { (*v).try_into()? },
    String => { v.parse()? }
);
try_into_field_value_impl!(v, i8,
    U8, U16, U32, U64, I8, I16, I32, I64 => { (*v).try_into()? },
    String => { v.parse()? }
);
try_into_field_value_impl!(v, i16,
    U8, U16, U32, U64, I8, I16, I32, I64 => { (*v).try_into()? },
    String => { v.parse()? }
);
try_into_field_value_impl!(v, i32,
    U8, U16, U32, U64, I8, I16, I32, I64 => { (*v).try_into()? },
    String => { v.parse()? }
);
try_into_field_value_impl!(v, i64,
    U8, U16, U32, U64, I8, I16, I32, I64 => { (*v).try_into()? },
    String => { v.parse()? }
);
try_into_field_value_impl!(v, bool,
    Bool => { (*v).try_into()? },
    String => { v.parse()? }
);
try_into_field_value_impl!(v, String, String => { v.to_string() });
