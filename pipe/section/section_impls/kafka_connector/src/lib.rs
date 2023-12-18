// Kafka source section
// CAUTION: ALPHA QUALITY CODE :) Use with caution.
use section::message::Message;
pub mod destination;

pub struct KafkaMessage {}

impl KafkaMessage {
    pub fn new() -> Self {
        Self {}
    }
}

impl std::fmt::Debug for KafkaMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KafkaMessage").finish()
    }
}

// I presume these are used for the Kafka source section
impl Message for KafkaMessage {
    fn origin(&self) -> &str {
        unimplemented!()
    }

    fn next(&mut self) -> section::message::Next<'_> {
        unimplemented!()
    }

    fn ack(&mut self) -> section::message::Ack {
        unimplemented!()
    }
}
