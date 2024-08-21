#[cfg(feature="section")]
mod bin;
#[cfg(feature="section")]
mod df;

#[derive(Debug, Clone, config::Config)]
pub struct Exec {
    command: String,
    args: String,
    ack_passthrough: bool,
    env: String,
    stream_binary: bool,
}