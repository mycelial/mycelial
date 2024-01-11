use section::message::{Chunk, Column, DataFrame, DataType, Message, ValueView};

pub mod destination;
pub mod source;

#[derive(Debug)]
pub(crate) struct FileDf {
    path: String,
}

impl DataFrame for FileDf {
    fn columns(&self) -> Vec<section::message::Column<'_>> {
        vec![Column::new(
            "file_path",
            DataType::Str,
            Box::new(std::iter::once(ValueView::Str(self.path.as_str()))),
        )]
    }
}

#[derive(Debug)]
pub(crate) struct FileMessage {
    inner: Option<Box<dyn DataFrame>>,
}

impl Message for FileMessage {
    fn origin(&self) -> &str {
        ""
    }

    fn next(&mut self) -> section::message::Next<'_> {
        let next = self.inner.take().map(Chunk::DataFrame);
        Box::pin(async move { Ok(next) })
    }

    fn ack(&mut self) -> section::message::Ack {
        Box::pin(async {})
    }
}

impl FileMessage {
    pub fn new(path: &str) -> Self {
        Self {
            inner: Some(Box::new(FileDf { path: path.into() })),
        }
    }
}
