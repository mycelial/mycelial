pub mod destination;
pub mod source;

use std::fmt::Display;

// should we just introduce additional method in message trait to indicate stream type?
#[derive(Debug)]
pub(crate) enum StreamType {
    DataFrame,
    BinStream,
}

impl Display for StreamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let desc = match self {
            StreamType::DataFrame => "arrow", // dataframe will be converted to arrow record batch
            StreamType::BinStream => "binary",
        };
        write!(f, "{}", desc)
    }
}
