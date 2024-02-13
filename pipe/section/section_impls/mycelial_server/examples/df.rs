use mycelial_server::destination;
use section::{
    dummy::DummySectionChannel,
    futures::SinkExt,
    message::{Chunk, Column, DataFrame, DataType, Message, Value, ValueView},
    section::Section,
    SectionError,
};
use stub::Stub;

// stream each value in vector as a separate dataframe
#[derive(Debug)]
struct Df {
    i64s: Vec<Value>,
    strs: Vec<String>,
    anys: Vec<Value>,
}

impl DataFrame for Df {
    fn columns(&self) -> Vec<section::message::Column<'_>> {
        vec![
            Column::new(
                "i64",
                DataType::I64,
                Box::new(self.i64s.iter().map(ValueView::from)),
            ),
            Column::new(
                "str",
                DataType::Str,
                Box::new(self.strs.iter().map(|v| ValueView::Str(v.as_str()))),
            ),
            Column::new(
                "any",
                DataType::Any,
                Box::new(self.anys.iter().map(Into::into)),
            ),
        ]
    }
}

#[derive(Debug)]
struct Msg {
    inner: Option<Df>,
}

impl Message for Msg {
    fn origin(&self) -> &str {
        "example_df"
    }

    fn next(&mut self) -> section::message::Next<'_> {
        let v = self.inner.take().map(|v| Chunk::DataFrame(Box::new(v)));
        Box::pin(async move { Ok(v) })
    }

    fn ack(&mut self) -> section::message::Ack {
        Box::pin(async {})
    }
}

#[tokio::main]
async fn main() {
    let dst = destination::Mycelial::new("http://localhost:7777/ingestion/", "topic");
    let (mut tx, rx) = section::futures::channel::mpsc::channel(1);
    let handle = tokio::spawn(dst.start(
        rx,
        Stub::<_, SectionError>::new(),
        DummySectionChannel::new(),
    ));
    let df = Df {
        i64s: vec![Value::I64(1), Value::Null, Value::I64(3)],
        strs: vec!["one", "two", "three"]
            .into_iter()
            .map(Into::into)
            .collect(),
        anys: vec![Value::I64(1), Value::Str("hello".into()), Value::Null],
    };
    tx.send(Box::new(Msg { inner: Some(df) })).await.unwrap();
    handle.await.ok();
}
