use pipe::{types::DynSection, config::Map};
use section::{command_channel::SectionChannel, SectionError};

pub fn source_ctor<S: SectionChannel>(config: &Map) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let message = config
        .get("message")
        .ok_or("hello world section requires 'message'")?
        .as_str()
        .ok_or("'message' should be a string")?;
    let interval_milis = config
        .get("interval_milis")
        .ok_or("hello world section requires 'interval_milis'")?
        .as_int()
        .ok_or("'interval_milis' should be an int")?;
    Ok(Box::new(hello_world::source::HelloWorld::new(message, interval_milis)))
}

pub fn destination_ctor<S: SectionChannel>(_: &Map) -> Result<Box<dyn DynSection<S>>, SectionError> {
    Ok(Box::new(hello_world::destination::HelloWorld::new()))
}
