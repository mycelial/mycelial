#[cfg(feature=section)]
pub mod source;

#[derive(Debug, Clone, config::Config)]

pub struct DirSourceConfig {
    path: PathBuf,
    pattern: String,
    start_after: String,
    interval: u64,
    stream_binary: bool,
}