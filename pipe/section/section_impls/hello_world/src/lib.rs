use section::{
    message::{Chunk, Column, DataFrame, DataType, Message, ValueView},
    SectionMessage,
};

pub mod destination;
pub mod source;

#[derive(Debug, Clone, PartialEq)]
pub struct HelloWorldPayload {
    /// message
    pub message: String,
    pub count: u32,
}

impl DataFrame for HelloWorldPayload {
    fn columns(&self) -> Vec<section::message::Column<'_>> {
        vec![Column::new(
            "message",
            DataType::Str,
            Box::new(std::iter::once(ValueView::from(&self.message))),
        ), Column::new(
            "count",
            DataType::U64,
            Box::new(std::iter::once(ValueView::from(&self.count))),
        )]
    }
}

#[derive(Debug)]
struct Once {
    inner: Option<Box<dyn DataFrame>>,
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
        Box::pin(async {})
    }
}

impl From<HelloWorldPayload> for SectionMessage {
    fn from(val: HelloWorldPayload) -> Self {
        Box::new(Once {
            inner: Some(Box::new(val)),
        })
    }
}
