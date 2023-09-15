mod section;
mod state;
mod command_channel;
mod message;

<<<<<<< HEAD
pub trait Section<Input, Output, CommandChannel> {
    type Error;
    type Future: Future<Output = Result<(), Self::Error>>;

    fn start(self, input: Input, output: Output, command_channel: CommandChannel) -> Self::Future;
}
=======
pub use section::Section;
pub use state::State;
pub use command_channel::{
    Command, SectionRequest, SectionChannel, RootChannel, WeakSectionChannel, ReplyTo
};
pub use message::Message;
pub use async_trait::async_trait;
>>>>>>> af66d55 (Initial)
