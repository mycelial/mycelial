mod raw_config;
mod ser;

pub use config_derive::Config;
use dyn_clone::DynClone;

pub mod prelude {
    pub use super::raw_config::{de::deserialize_into_config, RawConfig};
    pub use super::Config;
    pub use super::{Field, FieldType, FieldValue, Metadata, SectionIO};
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SectionIO {
    None,
    Bin,
    DataFrame,
    BinOrDataFrame,
}

impl SectionIO {
    pub fn is_none(self) -> bool {
        SectionIO::None == self
    }
}

pub fn clone_config(config: &dyn Config) -> Box<dyn Config> {
    dyn_clone::clone_box(config)
}

pub(crate) type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub trait Config: std::fmt::Debug + DynClone + std::any::Any + Send + Sync + 'static {
    fn name(&self) -> &str;

    fn input(&self) -> SectionIO;

    fn output(&self) -> SectionIO;

    fn fields(&self) -> Vec<Field<'_>>;

    fn get_field_value(&self, name: &str) -> Result<FieldValue<'_>, StdError>;

    fn set_field_value(&mut self, name: &str, value: FieldValue<'_>) -> Result<(), StdError>;

    fn strip_secrets(&mut self);

    fn validate_field(&self, field_name: &str, value: FieldValue<'_>) -> Result<(), StdError> {
        let mut iter = self
            .fields()
            .into_iter()
            .filter(|field| field.name == field_name);
        match iter.next() {
            Some(field) => {
                match field.ty {
                    FieldType::Usize => {
                        let _: usize = value.try_into()?;
                    }
                    FieldType::Bool => {
                        let _: bool = value.try_into()?;
                    }
                    FieldType::U8 => {
                        let _: u8 = value.try_into()?;
                    }
                    FieldType::U16 => {
                        let _: u16 = value.try_into()?;
                    }
                    FieldType::U32 => {
                        let _: u32 = value.try_into()?;
                    }
                    FieldType::U64 => {
                        let _: u64 = value.try_into()?;
                    }
                    FieldType::I8 => {
                        let _: i8 = value.try_into()?;
                    }
                    FieldType::I16 => {
                        let _: i16 = value.try_into()?;
                    }
                    FieldType::I32 => {
                        let _: i32 = value.try_into()?;
                    }
                    FieldType::I64 => {
                        let _: i64 = value.try_into()?;
                    }
                    // anything can be converted to String
                    FieldType::String => {}
                }
                Ok(())
            }
            None => Err("no such field")?,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FieldType {
    Usize,
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

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FieldValue<'a> {
    Usize(usize),
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
            FieldValue::Usize(_) => FieldType::Usize,
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
            FieldValue::Usize(v) => write!(f, "{v}"),
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

impl_from_ref!(usize, Usize, *);
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

impl_from_value!(usize, Usize);
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

macro_rules! try_into_field_value_impl {
    (
        $ty:ty,
        $var:ident,
        $own_arm:tt => $own_expr:tt,
        $special_case:tt => $special_expr:tt,
        $($arm:tt),+ => $arm_expr:tt,
        $($can_fail_arm:tt),+ => $can_fail_expr:tt
    )  => {
        impl TryInto<$ty> for FieldValue<'_> {
            type Error = StdError;

            fn try_into(self) -> Result<$ty, Self::Error> {
                match self {
                    FieldValue::$own_arm($var) => Ok(try_into_field_value_impl!(@expand_expr, $own_expr)),
                    FieldValue::$special_case($var) => Ok(try_into_field_value_impl!(@expand_expr, $special_expr)),
                    $(FieldValue::$arm($var) => Ok(try_into_field_value_impl!(@expand_expr, $arm_expr)),)*
                    $(FieldValue::$can_fail_arm($var) => Ok(try_into_field_value_impl!(@expand_expr, $can_fail_expr)),)*
                    _ => Err(format!("Can't convert {:?} into {}", self, stringify!($ty)))?
                }
            }
        }
    };
    (
        $ty:ty,
        $var:ident,
        $own_arm:tt => $own_expr:tt,
        $special_case:tt => $special_expr:tt,
        $($can_fail_arm:tt),+ => $can_fail_expr:tt
    )  => {
        impl TryInto<$ty> for FieldValue<'_> {
            type Error = StdError;

            fn try_into(self) -> Result<$ty, Self::Error> {
                match self {
                    FieldValue::$own_arm($var) => Ok(try_into_field_value_impl!(@expand_expr, $own_expr)),
                    FieldValue::$special_case($var) => Ok(try_into_field_value_impl!(@expand_expr, $special_expr)),
                    $(FieldValue::$can_fail_arm($var) => Ok(try_into_field_value_impl!(@expand_expr, $can_fail_expr)),)*
                    _ => Err(format!("Can't convert {:?} into {}", self, stringify!($ty)))?
                }
            }
        }
    };
    ($ty:ty, $var:ident, $own_arm:tt => $own_expr:tt, $special_case:tt => $special_expr:tt)  => {
        impl TryInto<$ty> for FieldValue<'_> {
            type Error = StdError;

            fn try_into(self) -> Result<$ty, Self::Error> {
                match self {
                    FieldValue::$own_arm($var) => Ok(try_into_field_value_impl!(@expand_expr, $own_expr)),
                    FieldValue::$special_case($var) => Ok(try_into_field_value_impl!(@expand_expr, $special_expr)),
                    _ => Err(format!("Can't convert {:?} into {}", self, stringify!($ty)))?
                }
            }
        }
    };
    ($ty:ty, $var:ident, $own_arm:tt => $own_expr:tt)  => {
        impl TryInto<$ty> for FieldValue<'_> {
            type Error = StdError;

            fn try_into(self) -> Result<$ty, Self::Error> {
                match self {
                    FieldValue::$own_arm($var) => Ok(try_into_field_value_impl!(@expand_expr, $own_expr)),
                    _ => Err(format!("Can't convert {:?} into {}", self, stringify!($ty)))?
                }
            }
        }
    };
    (@expand_expr, { $($token:tt)+ }) => {
        $($token)*
    }
}

try_into_field_value_impl!(usize, v,
    Usize => { v },
    String => { v.parse()? },
    U8, U16 => { v.into() },
    U32, U64, I8, I16, I32, I64 => { v.try_into()? }
);

try_into_field_value_impl!(u8, v,
    U8 => { v },
    String => { v.parse()? },
    U16, U32, U64, I8, I16, I32, I64 => { v.try_into()? }
);
try_into_field_value_impl!(u16, v,
    U16 => { v },
    String => { v.parse()? },
    U8 => { v.into() },
    U32, U64, I8, I16, I32, I64 => { v.try_into()? }
);
try_into_field_value_impl!(u32, v,
    U32 => { v },
    String => { v.parse()? },
    U8, U16 => { v.into() },
    U64, I8, I16, I32, I64 => { v.try_into()? }
);
try_into_field_value_impl!(u64, v,
    U64 => { v },
    String => { v.parse()? },
    U8, U16, U32 => { v.into() },
    I8, I16, I32, I64 => { v.try_into()? }
);
try_into_field_value_impl!(i8, v,
    I8 => { v },
    String => { v.parse()? },
    U8, U16, U32, U64, I16, I32, I64 => { v.try_into()? }
);
try_into_field_value_impl!(i16, v,
    I16 => { v },
    String => { v.parse()? },
    U8, I8 => { v.into() },
    U16, U32, U64, I32, I64 => { v.try_into()? }
);
try_into_field_value_impl!(i32, v,
    I32 => { v },
    String => { v.parse()? },
    U8, U16, I8, I16 => { v.into() },
    U32, U64, I64 => { v.try_into()? }
);
try_into_field_value_impl!(i64, v,
    I64 => { v },
    String => { v.parse()? },
    U8, U16, U32, I8, I16, I32 => { v.into() },
    U64 => { v.try_into()? }
);
try_into_field_value_impl!(bool, v,
    Bool => { v },
    String => { v.parse()? }
);
try_into_field_value_impl!(String, v, String => { v.to_string() });
