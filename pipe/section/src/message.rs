//! Section messaging

use std::future::Future;
use std::pin::Pin;

pub type FnAck = Pin<Box<dyn Future<Output=()> + Send + 'static>>;

pub struct Message<Payload> {
    pub origin: String,
    pub payload: Payload,
    ack: Option<FnAck>,
}

impl<P: std::fmt::Debug> std::fmt::Debug for Message<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Message")
         .field("origin", &self.origin)
         .field("payload", &self.payload)
         .finish()
    }
}

impl<P> Message<P> {
    pub fn new(origin: impl Into<String>, payload: impl Into<P>, ack: Option<FnAck>) -> Self {
        Self{
            origin: origin.into(),
            payload: payload.into(),
            ack,
        }
    }

    pub async fn ack(&mut self) {
        if let Some(ack) = self.ack.take() {
            ack.await;
        }
    }
}
