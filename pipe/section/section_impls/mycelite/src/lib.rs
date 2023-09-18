pub mod source;
pub mod destination;

use section::Message as _Message;
use arrow::record_batch::RecordBatch;

pub type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type Message = _Message<RecordBatch>;
