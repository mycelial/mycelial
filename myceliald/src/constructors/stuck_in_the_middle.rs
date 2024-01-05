use pipe::{config::Map, types::DynSection};
use section::{command_channel::SectionChannel, SectionError};

pub fn source_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    Ok(Box::new(stuck_in_the_middle::StuckInTheMiddle::new()))
}

pub fn destination_ctor<S: SectionChannel>(
    _: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    Ok(Box::new(stuck_in_the_middle::StuckInTheMiddle::new()))
}
