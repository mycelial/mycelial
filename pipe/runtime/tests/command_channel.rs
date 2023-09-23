use std::time::Duration;

use runtime::command_channel::{RootChannel, SectionRequest};
use section::{Command, RootChannel as _, SectionChannel as _, WeakSectionChannel};
use section::dummy::DummyState;
use tokio::time::timeout;

const TIMEOUT: Duration = Duration::from_millis(100);

#[tokio::test]
async fn test_command_channel() -> Result<(), Box<dyn std::error::Error>> {
    let mut root_chan = RootChannel::new();
    let mut section_chan = root_chan.section_channel(0).unwrap();

    let handle = tokio::spawn(async move {
        // ask root chan for state
        let state = timeout(TIMEOUT, section_chan.retrieve_state()).await;
        assert!(matches!(state, Ok(Ok(_))), "{:?}", state);
        let state = state.unwrap().unwrap();
        assert!(state.is_none());

        // ask to store state
        let state = DummyState::new();
        let res = timeout(TIMEOUT, section_chan.store_state(state)).await;
        assert!(matches!(res, Ok(Ok(_))));

        // send ack through weak ref to self
        let ack_chan = section_chan.weak_chan();
        tokio::spawn(async move { ack_chan.ack(Box::new("ack this")).await });

        let command = timeout(TIMEOUT, section_chan.recv()).await;
        assert!(matches!(command, Ok(Ok(_))));
        let command = command.unwrap().unwrap();
        assert!(matches!(command, Command::Ack(_)));
        let ack = match command {
            Command::Ack(a) => a,
            _ => unreachable!(),
        };
        assert_eq!(*ack.downcast::<&'static str>().unwrap(), "ack this");

        // log message
        let log_res = section_chan.log("hello").await;
        assert!(log_res.is_ok());

        // receive stop signal
        let cmd = section_chan.recv().await;
        assert!(cmd.is_ok());
        let cmd = cmd.unwrap();
        match cmd {
            Command::Stop => {
                Result::<_, Box<dyn std::error::Error + Send + Sync + 'static>>::Ok(())
            }
            _ => Err(format!("unexpected command: {:?}", cmd))?,
        }
    });

    // receive state request and respond with None
    let state_request = timeout(TIMEOUT, root_chan.recv()).await;
    assert!(matches!(state_request, Ok(Ok(_))));
    let state_request = state_request.unwrap().unwrap();
    let reply_result = state_request.reply_retrieve_state(None).await;
    assert!(reply_result.is_ok());

    // receive state store request and respond with ()
    let store_state_request = timeout(TIMEOUT, root_chan.recv()).await;
    assert!(matches!(store_state_request, Ok(Ok(_))));
    let store_state_request = store_state_request.unwrap().unwrap();
    let reply_result = store_state_request.reply_store_state().await;
    assert!(reply_result.is_ok());

    // receive request to log message
    let log_request = timeout(TIMEOUT, root_chan.recv()).await;
    assert!(matches!(log_request, Ok(Ok(_))));
    let log_request = log_request.unwrap().unwrap();
    assert!(matches!(log_request, SectionRequest::Log { .. }));
    let (id, log) = match log_request {
        section::SectionRequest::Log { id, message } => (id, message),
        _ => unreachable!(),
    };
    assert_eq!(id, 0);
    assert_eq!(log, "hello");

    // send stop signal
    let stop_res = timeout(TIMEOUT, root_chan.send(0, Command::Stop)).await;
    assert!(matches!(stop_res, Ok(Ok(_))), "{:?}", stop_res);

    let section_result = handle.await;
    assert!(matches!(section_result, Ok(Ok(_))));
    Ok(())
}
