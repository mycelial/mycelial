//! Runtime
//!
//! Pipe scheduling and peristance

use crate::{
    constructors,
    storage::{SqliteState, SqliteStorageHandle},
};
use pipe::{
    registry::{Constructor, Registry},
    scheduler::{Scheduler, SchedulerHandle},
};
use section::command_channel::SectionChannel;
//use mycelial_server;
//use mysql_connector;
//use postgres_connector;
//use snowflake;
//use sqlite_connector;
//use sqlite_physical_replication;
//use section::SectionChannel;

/// Setup & populate registry
fn setup_registry<S: SectionChannel>() -> Registry<S> {
    let arr: &[(&str, Constructor<S>)] = &[
        (
            "hello_world_destination",
            constructors::hello_world::destination_ctor,
        ),
        ("hello_world_source", constructors::hello_world::source_ctor),
        (
            "sqlite_connector_destination",
            constructors::sqlite_connector::destination_ctor,
        ),
        (
            "sqlite_connector_source",
            constructors::sqlite_connector::source_ctor,
        ),
        (
            "excel_connector_source",
            constructors::excel_connector::source_ctor,
        ),
        (
            "postgres_connector_destination",
            constructors::postgres_connector::destination_ctor,
        ),
        (
            "postgres_connector_source",
            constructors::postgres_connector::source_ctor,
        ),
        (
            "kafka_destination",
            constructors::kafka_connector::destination_ctor,
        ),
        ("mycelial_server_destination", constructors::mycelial_server::destination_ctor),
        //("mycelial_server_source", mycelial_server::source::constructor),
        //("mysql_connector_destination", mysql_connector::destination::constructor),
        //("snowflake_destination", snowflake::destination::constructor),
        //("snowflake_source", snowflake::source::constructor),
        //("sqlite_physical_replication_destination", sqlite_physical_replication::destination::constructor),
        //("sqlite_physical_replication_source", sqlite_physical_replication::source::constructor),
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
