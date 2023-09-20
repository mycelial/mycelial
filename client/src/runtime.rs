//! Runtime
//!
//! Pipe scheduling and peristance

use std::{future::Future, pin::Pin};

use pipe::{
    registry::{Constructor, Registry},
    scheduler::{Scheduler, SchedulerHandle}, storage::Storage, types::SectionError,
};
use section::State;

/// Setup & populate registry
fn setup_registry() -> Registry {
    let arr: &[(&str, Constructor)] = &[
      //("sqlite_source", sqlite::source::constructor),
      //("sqlite_destination", sqlite::destination::constructor),
      //("mycelite_source", mycelite::source::constructor),
      //("mycelite_destination", mycelite::destination::constructor),
      //("mycelial_net_source", mycelial_net::source::constructor),
      //(
      //    "mycelial_net_destination",
      //    mycelial_net::destination::constructor,
      //),
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

#[derive(Debug, Clone)]
pub struct NoopStorage {
}

#[derive(Debug, Clone)]
pub struct NoopState {
}

impl State for NoopState {
    fn new() -> Self {
        Self {}
    }

    fn get<T>(&self, key: &str) -> Option<T> {
        None
    }

    fn set<T>(&mut self, key: &str, value: T) {
    }
}

impl Storage<NoopState> for NoopStorage {
    fn store_state(
        &self,
        id: u64,
        section_id: u64,
        section_name: String,
        state: NoopState,
    ) -> Pin<Box<dyn Future<Output = Result<(), SectionError>> + Send + 'static>> {
        Box::pin(async { Ok(()) })
    }

    fn retrieve_state(
        &self,
        id: u64,
        section_id: u64,
        section_name: String,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NoopState>, SectionError>> + Send + 'static>> {
        Box::pin( async { Ok(None) })
    }
}

pub fn new() -> SchedulerHandle {
    let storage = NoopStorage{};
    Scheduler::new(setup_registry(), storage).spawn()
}
