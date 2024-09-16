#[cfg(feature = "section")]
pub mod destination;
#[cfg(feature = "section")]
pub(crate) mod message;
#[cfg(feature = "section")]
pub mod source;
#[cfg(feature = "section")]
pub(crate) mod stateful_query;
#[cfg(feature = "section")]
pub(crate) type Result<T, E = section::SectionError> = std::result::Result<T, E>;

#[derive(Debug, Clone, config::Configuration)]
#[section(output=dataframe)]
pub struct PostgresSource {
    host: String,
    port: u16,
    user: String,
    #[field_type(password)]
    password: String,
    database: String,
    origin: String,
    poll_interval: u64,
    #[field_type(text_area)]
    query: String,
}

impl PostgresSource {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host: impl Into<String>,
        port: u16,
        user: impl Into<String>,
        password: impl Into<String>,
        database: impl Into<String>,
        origin: impl Into<String>,
        poll_interval: u64,
        query: impl Into<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            user: user.into(),
            password: password.into(),
            database: database.into(),
            origin: origin.into(),
            poll_interval,
            query: query.into(),
        }
    }
}

impl Default for PostgresSource {
    fn default() -> Self {
        Self::new(
            "localhost",
            5432,
            "postgres",
            "",
            "postgres",
            "test",
            30,
            "select * from test",
        )
    }
}

#[derive(Debug, Clone, config::Configuration)]
#[section(input=dataframe)]
pub struct PostgresDestination {
    host: String,
    port: u16,
    user: String,
    #[field_type(password)]
    password: String,
    database: String,
    schema: String,
    truncate: bool,
    max_parameters: u32,
}

impl PostgresDestination {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host: impl Into<String>,
        port: u16,
        user: impl Into<String>,
        password: impl Into<String>,
        database: impl Into<String>,
        schema: impl Into<String>,
        truncate: bool,
        max_parameters: u32,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            user: user.into(),
            password: password.into(),
            database: database.into(),
            schema: schema.into(),
            truncate,
            max_parameters,
        }
    }
}

impl Default for PostgresDestination {
    fn default() -> Self {
        Self {
            host: "localhost".into(),
            port: 5432,
            user: "postgres".into(),
            password: "".into(),
            database: "postgres".into(),
            schema: "PUBLIC".into(),
            truncate: false,
            max_parameters: 1 << 15,
        }
    }
}
