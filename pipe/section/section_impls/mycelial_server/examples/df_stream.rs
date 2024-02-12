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
struct DfStream {
    i64s: Vec<i64>,
    strs: Vec<String>,
    anys: Vec<Value>,
    pos: usize,
}

#[derive(Debug)]
struct Df {
    i64: i64,
    str: String,
    any: Value,
}

impl DataFrame for Df {
    fn columns(&self) -> Vec<section::message::Column<'_>> {
        vec![
            Column::new(
                "i64",
                DataType::I64,
                Box::new(std::iter::once(self.i64).map(ValueView::I64)),
            ),
            Column::new(
                "str",
                DataType::Str,
                Box::new(std::iter::once(self.str.as_str()).map(ValueView::Str)),
            ),
            Column::new(
                "any",
                DataType::Any,
                Box::new(std::iter::once(&self.any).map(Into::into)),
            ),
        ]
    }
}

impl Message for DfStream {
    fn origin(&self) -> &str {
        "example_df_stream"
    }

    fn next(&mut self) -> section::message::Next<'_> {
        if self.pos >= self.i64s.len() {
            Box::pin(async { Ok(None) })
        } else {
            let df = Box::new(Df {
                i64: self.i64s[self.pos],
                str: self.strs[self.pos].clone(),
                any: self.anys[self.pos].clone(),
            });
            self.pos += 1;
            Box::pin(async move { Ok(Some(Chunk::DataFrame(df))) })
        }
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
    let msg = DfStream {
        i64s: vec![1, 2, 3],
        strs: vec!["one", "two", "three"]
            .into_iter()
            .map(Into::into)
            .collect(),
        anys: vec![Value::I64(1), Value::Str("hello".into()), Value::Null],
        pos: 0,
    };
    tx.send(Box::new(msg)).await.unwrap();
    handle.await.ok();
}
