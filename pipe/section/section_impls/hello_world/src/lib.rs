use section::{message::{Message, DataFrame, Column, ValueView, Chunk}, SectionMessage};

pub mod source;
pub mod destination;

#[derive(Debug, Clone, PartialEq)]
pub struct HelloWorldPayload {
    /// message
    pub message: String,
}

impl DataFrame for HelloWorldPayload {
    fn columns(&self) -> Vec<section::message::Column<'_>> {
        vec![
            Column::new("message", Box::new(std::iter::once(ValueView::from(&self.message))))
        ]
    }
}

#[derive(Debug)]
struct Once {
    inner: Option<Box<dyn DataFrame>>
}

impl Message for Once {
    fn origin(&self) -> &str {
        "hello world"
    }

    fn next(&mut self) -> section::message::Next<'_> {
        let v = self.inner.take().map(Chunk::DataFrame);
        Box::pin(async move { Ok(v) })
    }

    fn ack(&mut self) -> section::message::Ack {
        Box::pin(async {} )
    }
}

impl From<HelloWorldPayload> for SectionMessage {
    fn from(val: HelloWorldPayload) -> Self {
        Box::new(Once{ inner: Some(Box::new(val)) })
    }
}
