//! Runtime
//!
//! Pipe scheduling and peristance

use crate::storage::SqliteStorageHandle;
use pipe::{
    registry::{Constructor, Registry},
    scheduler::{Scheduler, SchedulerHandle},
    sections::mycelial_net,
    sections::sqlite_physical_replication,
    sections::sqlite_connector,
};
use section::State;

/// Setup & populate registry
fn setup_registry<S: State>() -> Registry<S> {
    let arr: &[(&str, Constructor<S>)] = &[
        ("sqlite_connector_source", sqlite_connector::source::constructor),
        ("sqlite_connector_destination", sqlite_connector::destination::constructor),
        ("sqlite_physical_replication", sqlite_physical_replication::source::constructor),
        ("sqlite_physical_repl_destination", sqlite_physical_replication::destination::constructor),
        ("mycelial_net_source", mycelial_net::source::constructor),
        (
            "mycelial_net_destination",
            mycelial_net::destination::constructor,
        ),
        //("kafka_source", kafka::source::constructor),
        //("snowflake_source", snowflake::source::constructor),
        //("snowflake_destination", snowflake::destination::constructor),
        //("mycelite_source", mycelite::source::constructor),
        //("mycelite_destination", mycelite::destination::constructor),
        //("postgres_source", postgres::source::constructor),
    ];
    arr.iter()
        .fold(Registry::new(), |mut acc, &(section_name, constructor)| {
            acc.register_section(section_name, constructor);
            acc
        })
}

pub fn new(storage: SqliteStorageHandle) -> SchedulerHandle {
    Scheduler::new(setup_registry(), storage).spawn()
}
