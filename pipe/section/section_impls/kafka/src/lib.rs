use section::Message as _Message;

pub mod destination;

type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type Message = _Message<OwnedMessage>;

use rdkafka::message::OwnedMessage;
