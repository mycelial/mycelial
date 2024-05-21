pub use config_derive::Config;

pub mod prelude {
    pub use super::Config as _;
    pub use super::{Field, FieldType, Metadata, SectionIO};
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
pub struct Field {
    pub name: &'static str,
    pub ty: FieldType,
    pub metadata: Metadata,
}
