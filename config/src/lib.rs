pub use config_derive::SectionConfig;

pub trait Config {
    fn fields(&self) -> Vec<Field>;
}

#[derive(Debug, PartialEq)]
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
