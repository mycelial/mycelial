use pipe::{config::Map, types::DynSection};
use section::{command_channel::SectionChannel, SectionError};

/// # Config example:
/// ```toml
/// [[section]]
/// name = "mycelial_net"
/// endpoint = "http://localhost:7777/ingestion"
/// token = "token"
/// topic = "some_topic"
/// ```
pub fn source_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let endpoint = config
        .get("endpoint")
        .ok_or("mycelial net section requires 'endpoint' url")?
        .as_str()
        .ok_or("endpoint should be string")?;
    let token = config
        .get("token")
        .ok_or("mycelian net section requires 'token'")?
        .as_str()
        .ok_or("token should be string")?;
    // FIXME: validate topic is not empty
    let topic = config
        .get("topic")
        .ok_or("mycelian net section requires 'topic'")?
        .as_str()
        .ok_or("token should be string")?;
    Ok(Box::new(mycelial_server::source::Mycelial::new(
        endpoint, token, topic,
    )))
}

/// # Config example:
/// ```toml
/// [[section]]
/// name = "mycelial_net"
/// endpoint = "http://localhost:7777/ingestion"
/// token = "token"
/// ```
pub fn destination_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let endpoint = config
        .get("endpoint")
        .ok_or("mycelial net section requires 'endpoint' url")?
        .as_str()
        .ok_or("endpoint should be string")?;
    let token = config
        .get("token")
        .ok_or("mycelial net section requires 'token'")?
        .as_str()
        .ok_or("token should be string")?;
    let topic = config
        .get("topic")
        .ok_or("mycelial net section requires 'topic'")?
        .as_str()
        .ok_or("topic should be string")?;
    Ok(Box::new(mycelial_server::destination::Mycelial::new(
        endpoint, token, topic,
    )))
}
