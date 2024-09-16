use section::{message::Message, SectionMessage};

pub mod regex;
pub mod time_nanos;

#[derive(Debug)]
pub(crate) struct OriginTransformMsg {
    pub origin: String,
    pub inner: SectionMessage,
}

impl OriginTransformMsg {
    pub fn new(origin: String, inner: SectionMessage) -> Self {
        Self { origin, inner }
    }
}

impl Message for OriginTransformMsg {
    fn ack(&mut self) -> section::message::Ack {
        self.inner.ack()
    }

    fn next(&mut self) -> section::message::Next<'_> {
        self.inner.next()
    }

    fn origin(&self) -> &str {
        self.origin.as_str()
    }
}
