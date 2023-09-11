//! Section interface
use std::future::Future;

pub trait Section<Input, Output, CommandChannel> {
    type Error;
    type Future: Future<Output = Result<(), Self::Error>>;

    fn start(self, input: Input, output: Output, command_channel: CommandChannel) -> Self::Future;
}
