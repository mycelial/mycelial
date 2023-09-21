//! Runtime
//!
//! Pipe scheduling and peristance

use std::{future::Future, pin::Pin};

use pipe::{
    registry::{Constructor, Registry},
    scheduler::{Scheduler, SchedulerHandle},
    storage::Storage,
    types::SectionError,
    sections::mycelite,
    sections::mycelial_net,
    sections::sqlite,
};
use section::State;

use crate::storage::{SqliteStorage, SqliteStorageHandle};

/// Setup & populate registry
fn setup_registry<S: State>() -> Registry<S> {
    let arr: &[(&str, Constructor<S>)] = &[
        ("sqlite_source", sqlite::source::constructor),
        ("sqlite_destination", sqlite::destination::constructor),
        ("mycelite_source", mycelite::source::constructor),
        ("mycelite_destination", mycelite::destination::constructor),
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
    arr.iter() .fold(Registry::new(), |mut acc, &(section_name, constructor)| {
            acc.register_section(section_name, constructor);
            acc
        })
}

pub fn new(storage: SqliteStorageHandle) -> SchedulerHandle {
    Scheduler::new(setup_registry(), storage).spawn()
}
