#[cfg(feature = "section")]
mod section;

#[derive(Debug, Clone, config::Configuration)]
#[section(input=bin_or_dataframe, output=bin_or_dataframe)]
pub struct Exec {
    command: String,
    args: String,
    ack_passthrough: bool,
    env: String,
    stream_binary: bool,
}

impl Exec {
    fn new(
        command: impl Into<String>,
        args: impl Into<String>,
        env: impl Into<String>,
        ack_passthrough: bool,
        stream_binary: bool,
    ) -> Self {
        Self {
            command: command.into(),
            args: args.into(),
            env: env.into(),
            ack_passthrough,
            stream_binary,
        }
    }
}


impl Default for Exec {
    fn default() -> Self {
        Self::new (
            "echo",
            "foo",
            "FOO=BAR",
            true,
            false,
        )
    }
}