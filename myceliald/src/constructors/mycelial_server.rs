use pipe::{config::Map, types::DynSection};
use section::{command_channel::SectionChannel, SectionError};

/// # Config example:
/// ```toml
/// [[section]]
/// name = "mycelial_net"
/// endpoint = "http://localhost:7777/ingestion"
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
    // FIXME: validate topic is not empty
    let topic = config
        .get("topic")
        .ok_or("mycelial net section requires 'topic'")?
        .as_str()
        .ok_or("topic should be string")?;
    let client_id = config
        .get("client_id")
        .ok_or("mycelial net section requires 'client_id'")?
        .as_str()
        .ok_or("client_id should be string")?;
    let client_secret = config
        .get("client_secret")
        .ok_or("mycelial net section requires 'client_secret'")?
        .as_str()
        .ok_or("client_secret should be string")?;
    Ok(Box::new(mycelial_server::source::Mycelial::new(
        endpoint,
        topic,
        client_id,
        client_secret,
    )))
}

/// # Config example:
/// ```toml
/// [[section]]
/// name = "mycelial_net"
/// endpoint = "http://localhost:7777/ingestion"
/// ```
pub fn destination_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let endpoint = config
        .get("endpoint")
        .ok_or("mycelial net section requires 'endpoint' url")?
        .as_str()
        .ok_or("endpoint should be string")?;
    let topic = config
        .get("topic")
        .ok_or("mycelial net section requires 'topic'")?
        .as_str()
        .ok_or("topic should be string")?;
    let client_id = config
        .get("client_id")
        .ok_or("mycelial net section requires 'client_id'")?
        .as_str()
        .ok_or("client_id should be string")?;
    let client_secret = config
        .get("client_secret")
        .ok_or("mycelial net section requires 'client_secret'")?
        .as_str()
        .ok_or("client_secret should be string")?;
    Ok(Box::new(mycelial_server::destination::Mycelial::new(
        endpoint,
        topic,
        client_id,
        client_secret,
    )))
}
