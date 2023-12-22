use std::time::Duration;

use pipe::{config::{Value, Map}, types::DynSection};
use section::{command_channel::SectionChannel, SectionError};

pub fn source_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let username = config
        .get("username")
        .ok_or("username required")?
        .as_str()
        .ok_or("'username' should be a string")?;
    let password = config
        .get("password")
        .ok_or("password required")?
        .as_str()
        .ok_or("'password' should be a string")?;
    let role = config
        .get("role")
        .ok_or("role required")?
        .as_str()
        .ok_or("'role' should be a string")?;
    let account_identifier = config
        .get("account_identifier")
        .ok_or("account_identifier required")?
        .as_str()
        .ok_or("'account_identifier' should be a string")?;
    let warehouse = config
        .get("warehouse")
        .ok_or("warehouse required")?
        .as_str()
        .ok_or("'warehouse' should be a string")?;
    let database = config
        .get("database")
        .ok_or("database required")?
        .as_str()
        .ok_or("'database' should be a string")?;
    let schema = config
        .get("schema")
        .ok_or("schema required")?
        .as_str()
        .ok_or("'schema' should be a string")?;
    let query = config
        .get("query")
        .ok_or("query required")?
        .as_str()
        .ok_or("'query' should be a string")?;
    let delay = match config.get("delay").ok_or("snowflake source requires 'delay'")? {
        Value::Int(i) => *i,
        Value::String(s) => s.parse()?,
        _ => Err("delay should be int")?,
    };
    Ok(Box::new(snowflake::source::SnowflakeSource::new(
        username,
        password,
        role,
        account_identifier,
        warehouse,
        database,
        schema,
        query,
        Duration::from_secs(delay as u64),
    )))
}

pub fn destination_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let username = config
        .get("username")
        .ok_or("username required")?
        .as_str()
        .ok_or("'username' should be a string")?;
    let password = config
        .get("password")
        .ok_or("password required")?
        .as_str()
        .ok_or("'password' should be a string")?;
    let role = config
        .get("role")
        .ok_or("role required")?
        .as_str()
        .ok_or("'role' should be a string")?;
    let account_identifier = config
        .get("account_identifier")
        .ok_or("account_identifier required")?
        .as_str()
        .ok_or("'account_identifier' should be a string")?;
    let warehouse = config
        .get("warehouse")
        .ok_or("warehouse required")?
        .as_str()
        .ok_or("'warehouse' should be a string")?;
    let database = config
        .get("database")
        .ok_or("database required")?
        .as_str()
        .ok_or("'database' should be a string")?;
    let schema = config
        .get("schema")
        .ok_or("schema required")?
        .as_str()
        .ok_or("'schema' should be a string")?;
    Ok(Box::new(snowflake::destination::SnowflakeDestination::new(
        username,
        password,
        role,
        account_identifier,
        warehouse,
        database,
        schema,
    )))
}
