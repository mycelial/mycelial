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

/// Setup & populate registry
fn setup_registry<S: SectionChannel>() -> Registry<S> {
    let arr: &[(&str, Constructor<S>)] = &[
        (
            "hello_world_destination",
            constructors::hello_world::destination_ctor,
        ),
        ("hello_world_source", constructors::hello_world::source_ctor),
        (
            "tagging_transformer",
            constructors::tagging_transformer::transform_ctor,
        ),
        (
            "typecast_transformer",
            constructors::typecast_transformer::transformer,
        ),
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
        (
            "mycelial_server_destination",
            constructors::mycelial_server::destination_ctor,
        ),
        (
            "mycelial_server_source",
            constructors::mycelial_server::source_ctor,
        ),
        (
            "snowflake_destination",
            constructors::snowflake::destination_ctor,
        ),
        ("snowflake_source", constructors::snowflake::source_ctor),
        (
            "mysql_connector_destination",
            constructors::mysql_connector::destination_ctor,
        ),
        (
            "mysql_connector_source",
            constructors::mysql_connector::source_ctor,
        ),
        ("file_source", constructors::file::source_ctor),
        ("file_destination", constructors::file::destination_ctor),
        ("dir_source", constructors::dir::source_ctor),
        ("exec", constructors::exec::exec_ctor),
        ("from_csv", constructors::csv_transform::source_ctor),
        ("to_csv", constructors::csv_transform::destination_ctor),
        (
            "origin_regex_transform",
            constructors::origin_transform::regex_ctor,
        ),
        (
            "origin_time_nanos_transform",
            constructors::origin_transform::time_nanos_ctor,
        ),
        ("s3_source", constructors::s3::source_ctor),
        ("s3_destination", constructors::s3::destination_ctor),
        (
            "redshift_loader_destination",
            constructors::redshift_loader::ctor,
        ),
        ("inspect", constructors::inspect::inspect_ctor),
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
