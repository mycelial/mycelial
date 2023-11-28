pub mod command_channel;
pub mod message;
pub mod section;
pub mod state;
pub mod dummy;
pub mod pretty_print;

use std::pin::Pin;

// re-export
pub use async_trait::async_trait;
pub use futures;

pub type SectionError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type SectionFuture = Pin<Box<dyn std::future::Future<Output=Result<(), SectionError>> + Send + 'static >>;
pub type SectionMessage = Box<dyn crate::message::Message>;
