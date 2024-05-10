use std::pin::pin;
use std::sync::Arc;

use section::command_channel::{Command, SectionChannel};
use section::futures::{self, Sink, SinkExt, Stream};
use section::futures::{FutureExt, StreamExt};
use section::section::Section;
use section::{
    message::{Ack, Chunk, Column, DataFrame, DataType, Message, Value},
    SectionError, SectionFuture, SectionMessage,
};

#[derive(Debug, Clone, Copy)]
pub enum TargetType {
    Int,
    Real,
    String,
}

impl From<TargetType> for DataType {
    fn from(val: TargetType) -> Self {
        match val {
            TargetType::Int => DataType::I64,
            TargetType::Real => DataType::F64,
            TargetType::String => DataType::Str,
        }
    }
}

impl TryFrom<&str> for TargetType {
    type Error = SectionError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "int" => Ok(TargetType::Int),
            "real" => Ok(TargetType::Real),
            "string" => Ok(TargetType::String),
            _ => Err(format!("unsupported type '{value}'"))?,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TargetColumn {
    // all columns
    All,
    // specific column
    Column(Arc<str>),
}

impl PartialEq<str> for TargetColumn {
    fn eq(&self, other: &str) -> bool {
        match self {
            Self::All => true,
            Self::Column(name) => (**name).eq(other),
        }
    }
}

impl From<&str> for TargetColumn {
    fn from(value: &str) -> Self {
        match value {
            "*" => TargetColumn::All,
            _ => TargetColumn::Column(Arc::from(value)),
        }
    }
}

#[derive(Debug)]
pub struct TypecastTransformer {
    target_type: TargetType,
    target_column: TargetColumn,
}

impl TypecastTransformer {
    pub fn new(target_type: &str, target_column: &str) -> Result<Self, SectionError> {
        Ok(Self {
            target_type: target_type.try_into()?,
            target_column: target_column.into(),
        })
    }
}

#[derive(Debug)]
pub struct TypecastMessage {
    inner: SectionMessage,
    target_type: TargetType,
    target_column: TargetColumn,
}

#[derive(Debug)]
struct TypecastDataframe {
    columns: Vec<(String, DataType, Vec<Value>)>,
}

impl DataFrame for TypecastDataframe {
    fn columns(&self) -> Vec<Column<'_>> {
        self.columns
            .iter()
            .map(|(name, data_type, values)| {
                Column::new(
                    name.as_str(),
                    *data_type,
                    Box::new(values.iter().map(Into::into)),
                )
            })
            .collect()
    }
}

impl TypecastMessage {
    fn new(inner: SectionMessage, target_type: TargetType, target_column: TargetColumn) -> Self {
        Self {
            inner,
            target_type,
            target_column,
        }
    }

    fn cast_df(&self, df: Box<dyn DataFrame>) -> Result<Box<dyn DataFrame>, SectionError> {
        let columns = df
            .columns()
            .into_iter()
            .map(|col| -> Result<_, SectionError> {
                let col_name: String = col.name().into();
                let col_datatype = col.data_type();
                let mut values = Vec::with_capacity(col.size_hint().0);
                let transform = self.target_column.eq(col_name.as_str());
                for value_view in col {
                    let value = match (transform, self.target_type) {
                        (false, _) => (&value_view).into(),
                        (true, TargetType::Int) => value_view.into_i64()?,
                        (true, TargetType::Real) => value_view.into_f64()?,
                        (true, TargetType::String) => value_view.into_str()?,
                    };
                    values.push(value)
                }
                let data_type = match transform {
                    true => self.target_type.into(),
                    false => col_datatype,
                };
                Ok((col_name, data_type, values))
            })
            .collect::<Result<Vec<(String, DataType, Vec<Value>)>, _>>()?;
        Ok(Box::new(TypecastDataframe { columns }))
    }
}

impl Message for TypecastMessage {
    fn origin(&self) -> &str {
        self.inner.origin()
    }

    fn next(&mut self) -> section::message::Next<'_> {
        Box::pin(async move {
            match self.inner.next().await {
                Ok(None) => Ok(None),
                Ok(Some(Chunk::Byte(_))) => {
                    Err("typecast transformer doesn't work with binary streams".into())
                }
                Ok(Some(Chunk::DataFrame(df))) => match self.cast_df(df) {
                    Ok(df) => Ok(Some(Chunk::DataFrame(df))),
                    Err(e) => Err(e),
                },
                Err(e) => Err(e),
            }
        })
    }

    fn ack(&mut self) -> Ack {
        self.inner.ack()
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for TypecastTransformer
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel + Send + Sync + 'static,
{
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(self, input: Input, output: Output, mut section_channel: SectionChan) -> Self::Future {
        Box::pin(async move {
            let mut input = pin!(input.fuse());
            let mut output = pin!(output);

            loop {
                futures::select! {
                    cmd = section_channel.recv().fuse() => {
                        if let Command::Stop = cmd? {
                            return Ok(())
                        }
                    }
                    msg = input.next() => {
                        let msg = match msg {
                            Some(msg) => msg,
                            None => Err("input stream closed")?
                        };
                        let msg = TypecastMessage::new(msg, self.target_type, self.target_column.clone());
                        output.send(Box::new(msg)).await?;
                    }
                }
            }
        })
    }
}
