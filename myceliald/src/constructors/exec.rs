use pipe::{
    config::{Map, Value},
    types::DynSection,
};
use section::{command_channel::SectionChannel, SectionError};

pub fn exec_ctor<S: SectionChannel>(config: &Map) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let command = config
        .get("command")
        .ok_or("exec required 'command' field")?
        .as_str()
        .ok_or("'command' must be a string")?;
    let args = match config.get("args") {
        None => None,
        Some(Value::String(s)) => Some(s.as_str()),
        Some(_) => Err("'args' must be a string")?,
    };
    let row_as_args = match config
        .get("row_as_args")
        .ok_or("exec requires 'row_as_args' field")?
    {
        Value::String(maybe_bool) => maybe_bool.to_lowercase() == "true",
        Value::Bool(b) => *b,
        _ => Err("'row_as_args' must be a bool")?,
    };
    let ack_passthrough = match config
        .get("ack_passthrough")
        .ok_or("exec requires 'ack_passthrough' field")?
    {
        Value::String(maybe_bool) => maybe_bool.to_lowercase() == "true",
        Value::Bool(b) => *b,
        _ => Err("'ack_passthrough' must be a bool")?,
    };
    let env: Vec<_> = match config.get("env") {
        Some(Value::String(s)) => s
            .split(',')
            .filter(|v| !v.is_empty())
            .map(
                |pair| match *pair.trim().splitn(2, '=').collect::<Vec<_>>().as_slice() {
                    [k] => (k, ""),
                    [k, v] => (k, v),
                    _ => unreachable!(),
                },
            )
            .collect(),
        Some(_) => Err("'env' must be a string")?,
        None => vec![],
    };
    Ok(Box::new(exec::Exec::new(
        command,
        args,
        row_as_args,
        ack_passthrough,
        env.as_slice(),
    )?))
}
