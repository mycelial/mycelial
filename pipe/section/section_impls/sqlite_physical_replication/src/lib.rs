pub mod destination;
pub mod source;

use arrow::record_batch::RecordBatch;
use section::Message as _Message;

pub type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type Message = _Message<RecordBatch>;
