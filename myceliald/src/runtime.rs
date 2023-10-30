//! Runtime
//!
//! Pipe scheduling and peristance

use crate::storage::{SqliteState, SqliteStorageHandle};
use pipe::{
    registry::{Constructor, Registry},
    scheduler::{Scheduler, SchedulerHandle},
    sections::{
        hello_world, kafka, mycelial_server, snowflake, sqlite_connector,
        sqlite_physical_replication, excel_connector
    },
};
use section::SectionChannel;

/// Setup & populate registry
fn setup_registry<S: SectionChannel>() -> Registry<S> {
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
            "sqlite_physical_replication_source",
            sqlite_physical_replication::source::constructor,
        ),
        (
            "sqlite_physical_replication_destination",
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
        ("hello_world_source", hello_world::source::constructor),
        (
            "hello_world_destination",
            hello_world::destination::constructor,
        ),
        (
            "excel_connector_source",
            excel_connector::source::constructor,
        ),
        ("kafka_destination", kafka::destination::constructor),
        ("snowflake_source", snowflake::source::constructor),
        ("snowflake_destination", snowflake::destination::constructor),
    ];
    arr.iter()
        .fold(Registry::new(), |mut acc, &(section_name, constructor)| {
            acc.register_section(section_name, constructor);
            acc
        })
}

pub fn new(storage: SqliteStorageHandle) -> SchedulerHandle {
    Scheduler::<_, pipe::command_channel::RootChannel<SqliteState>>::new(setup_registry(), storage)
        .spawn()
}
