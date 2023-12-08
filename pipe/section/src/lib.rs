pub mod command_channel;
pub mod dummy;
pub mod message;
pub mod pretty_print;
pub mod section;
pub mod state;

use std::pin::Pin;

// re-export
pub use async_trait::async_trait;
pub use futures;
pub use rust_decimal as decimal;
pub use time;
pub use uuid;

pub type SectionError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type SectionFuture =
    Pin<Box<dyn std::future::Future<Output = Result<(), SectionError>> + Send + 'static>>;
pub type SectionMessage = Box<dyn crate::message::Message>;
