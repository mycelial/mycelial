//! Runtime
//!
//! Pipe scheduling and peristance

use exp2::dynamic_pipe::section_impls::postgres;
use exp2::dynamic_pipe::{
    registry::{Constructor, Registry},
    scheduler::{Scheduler, SchedulerHandle},
    section_impls::{kafka, mycelial_net, mycelite, snowflake, sqlite},
};

use crate::storage::SqliteStorageHandle;

/// Setup & populate registry
fn setup_registry() -> Registry {
    let arr: &[(&str, Constructor)] = &[
        ("sqlite_source", sqlite::source::constructor),
        ("sqlite_destination", sqlite::destination::constructor),
        ("mycelial_net_source", mycelial_net::source::constructor),
        (
            "mycelial_net_destination",
            mycelial_net::destination::constructor,
        ),
        ("kafka_source", kafka::source::constructor),
        ("snowflake_source", snowflake::source::constructor),
        ("snowflake_destination", snowflake::destination::constructor),
        ("mycelite_source", mycelite::source::constructor),
        ("mycelite_destination", mycelite::destination::constructor),
        ("postgres_source", postgres::source::constructor),
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
