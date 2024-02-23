use std::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use runtime::{
    command_channel::{RootChannel, SectionChannel},
    config::{Config, Map, Value},
    registry::Registry,
    scheduler::{PipeStatus, Scheduler},
    storage::Storage,
    types::{DynSection, SectionFuture},
};
use section::{
    command_channel::SectionChannel as SectionChannelTrait, dummy::DummyState, section::Section,
    SectionError,
};

#[derive(Debug)]
struct NoopStorage {}

impl Storage<DummyState> for NoopStorage {
    fn store_state(
        &self,
        _: u64,
        _: DummyState,
    ) -> Pin<Box<dyn Future<Output = Result<(), SectionError>> + Send + 'static>> {
        Box::pin(async { Ok(()) })
    }

    fn retrieve_state(
        &self,
        _: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Option<DummyState>, SectionError>> + Send + 'static>>
    {
        Box::pin(async { Ok(None) })
    }
}

static SECTION_COUNTER: AtomicU64 = AtomicU64::new(0);

struct Counter {}

impl Counter {
    fn new() -> Self {
        SECTION_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self {}
    }
}

impl Drop for Counter {
    fn drop(&mut self) {
        SECTION_COUNTER.fetch_sub(1, Ordering::Relaxed);
    }
}

struct TestSection {
    delay: u64,
}

impl<Input, Output, SectionChan: SectionChannelTrait> Section<Input, Output, SectionChan>
    for TestSection
{
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(self, _: Input, _: Output, mut section_chan: SectionChan) -> Self::Future {
        Box::pin(async move {
            let _counter = Counter::new();
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(self.delay)) => {},
                _ = section_chan.recv() => {},
            }
            Ok(())
        })
    }
}

fn test_section_ctor(
    config: &Map,
) -> Result<Box<dyn DynSection<SectionChannel<DummyState>>>, SectionError> {
    let delay = config.get("delay").unwrap().as_int().unwrap() as u64;
    Ok(Box::new(TestSection { delay }))
}

// Scenario:
// spawn pipe
// section initializes counter after first poll, counter increments global atomic, after than
// section sleeps for period of time defined through section config
// counter on drop decreases global atomic
// global atomic allows to understand when section is alive
#[tokio::test]
async fn test_scheduler_restart() {
    let storage = NoopStorage {};
    let mut registry = Registry::<SectionChannel<DummyState>>::new();
    registry.register_section("test", test_section_ctor);
    let scheduler_handle =
        Scheduler::<NoopStorage, RootChannel<DummyState>>::new(registry, storage)
            .with_restart_delay(Duration::from_millis(100))
            .spawn();

    let mut section_config = Map::new();
    section_config.insert("delay".into(), Value::Int(100));
    section_config.insert("name".into(), Value::String("test".into()));

    let res = scheduler_handle
        .add_pipe(1, Config::new(vec![section_config]))
        .await;
    assert!(res.is_ok(), "failed to add pipe: {:?}", res);
    assert_eq!(1, SECTION_COUNTER.load(Ordering::Relaxed));
    let status = scheduler_handle.list_status().await;
    assert!(status.is_ok(), "failed to list status: {:?}", status);
    let status = status.unwrap();
    assert_eq!(status, vec![(1, PipeStatus::Running)]);

    // wait for section to shutdown
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(0, SECTION_COUNTER.load(Ordering::Relaxed));
    let status = scheduler_handle.list_status().await;
    assert!(status.is_ok(), "failed to list status: {:?}", status);
    let status = status.unwrap();
    assert_eq!(status, vec![(1, PipeStatus::Restarting)]);

    // wait for restart timeout
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(1, SECTION_COUNTER.load(Ordering::Relaxed));
    let status = scheduler_handle.list_status().await;
    assert!(status.is_ok(), "failed to list status: {:?}", status);
    let status = status.unwrap();
    assert_eq!(status, vec![(1, PipeStatus::Running)]);

    // update section config, reduce sleep duration
    let mut section_config = Map::new();
    section_config.insert("delay".into(), Value::Int(50));
    section_config.insert("name".into(), Value::String("test".into()));

    let res = scheduler_handle
        .add_pipe(1, Config::new(vec![section_config]))
        .await;
    assert!(res.is_ok(), "failed to add pipe: {:?}", res);

    assert_eq!(1, SECTION_COUNTER.load(Ordering::Relaxed));
    let status = scheduler_handle.list_status().await;
    assert!(status.is_ok(), "failed to list status: {:?}", status);
    let status = status.unwrap();
    assert_eq!(status, vec![(1, PipeStatus::Running)]);

    // wait for section with updated config to shutdown
    tokio::time::sleep(Duration::from_millis(75)).await;
    assert_eq!(0, SECTION_COUNTER.load(Ordering::Relaxed));
    let status = scheduler_handle.list_status().await;
    assert!(status.is_ok(), "failed to list status: {:?}", status);
    let status = status.unwrap();
    assert_eq!(status, vec![(1, PipeStatus::Restarting)]);
}
