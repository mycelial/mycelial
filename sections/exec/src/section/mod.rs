pub mod bin;
pub mod df;
use section::prelude::*;
use crate::Exec;

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Exec
where
    Input: SectionStream,
    Output: SectionSink,
    SectionChan: SectionChannel,
{
    type Future = SectionFuture;
    type Error = SectionError;

    fn start(self, input: Input, output: Output, section_channel: SectionChan) -> Self::Future {
        let env = self.env
            .split(',')
            .filter(|v| !v.is_empty())
            .map(
                |pair| match *pair.trim().splitn(2, '=').collect::<Vec<_>>().as_slice() {
                    [k] => (k, ""),
                    [k, v] => (k, v),
                    _ => unreachable!(),
                },
            )
            .collect();
        match self.stream_binary {
            true => {
                bin::ExecBin::new({
                    &self.command,
                    &self.
                }).start(input, output, section_channel)
            },
            false => {
                df::ExecDf::new( 
                    input,
                    output,
                    section_channel
                )
            }
        }
    }
}