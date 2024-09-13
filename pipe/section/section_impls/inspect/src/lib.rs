#[cfg(feature="section")]
mod section;

#[derive(Debug, Default, Clone, config::Configuration)]
#[section(input=bin_or_dataframe, output=bin_or_dataframe)]
pub struct Inspect {}