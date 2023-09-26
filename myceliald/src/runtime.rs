//! Runtime
//!
//! Pipe scheduling and peristance

use crate::storage::SqliteStorageHandle;
use pipe::{
    registry::{Constructor, Registry},
    scheduler::{Scheduler, SchedulerHandle},
    sections::mycelial_server,
    sections::sqlite_connector,
    sections::sqlite_physical_replication,
};
use section::State;

/// Setup & populate registry
fn setup_registry<S: State>() -> Registry<S> {
    let arr: &[(&str, Constructor<S>)] = &[
        (
            "sqlite_connector_source",
            sqlite_connector::source::constructor,
        ),
        (
            "sqlite_connector_destination",
            sqlite_connector::destination::constructor,
        ),
        (
            "sqlite_physical_replication",
            sqlite_physical_replication::source::constructor,
        ),
        (
            "sqlite_physical_repl_destination",
            sqlite_physical_replication::destination::constructor,
        ),
        (
            "mycelial_server_source",
            mycelial_server::source::constructor,
        ),
        (
            "mycelial_server_destination",
            mycelial_server::destination::constructor,
        ),
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
