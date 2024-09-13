#[cfg(feature = "section")]
pub mod destination;
#[cfg(feature = "section")]
pub mod source;

#[derive(Debug, Clone, config::Configuration)]
#[section(input=bin, output=dataframe)]
pub struct FromCsv {
    batch_size: usize,
}

impl Default for FromCsv {
    fn default() -> Self {
        Self { batch_size: 512 }
    }
}

impl FromCsv {
    pub fn new(batch_size: usize) -> Self {
        Self { batch_size }
    }
}

#[derive(Debug, Clone, config::Configuration)]
#[section(input=dataframe, output=bin)]
pub struct ToCsv {
    buf_size: usize,
}

impl Default for ToCsv {
    fn default() -> Self {
        Self { buf_size: 4096 }
    }
}
