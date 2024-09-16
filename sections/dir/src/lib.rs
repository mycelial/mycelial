#[cfg(feature = "section")]
pub mod source;

#[derive(Debug, Clone, config::Configuration)]
#[section(output=bin_or_dataframe)]
pub struct DirSource {
    path: String,
    pattern: String,
    start_after: String,
    interval: u64,
    stream_binary: bool,
}

impl DirSource {
    pub fn new(
        path: String,
        pattern: String,
        start_after: String,
        interval: u64,
        stream_binary: bool,
    ) -> Self {
        Self {
            path,
            pattern,
            start_after,
            interval,
            stream_binary,
        }
    }
}

impl Default for DirSource {
    fn default() -> Self {
        Self {
            path: "".into(),
            pattern: "".into(),
            start_after: "".into(),
            interval: 30,
            stream_binary: false,
        }
    }
}
