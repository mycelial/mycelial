#[cfg(feature = "section")]
pub mod source;

#[derive(Debug, Clone, config::Configuration)]
#[section(output=dataframe)]
pub struct Excel {
    path: String,
    sheets: String,
    stringify: bool,
}

impl Default for Excel {
    fn default() -> Self {
        Self {
            path: "".into(),
            sheets: "*".into(),
            stringify: false,
        }
    }
}
