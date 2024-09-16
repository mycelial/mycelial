#[cfg(feature = "section")]
pub mod destination;

#[derive(Debug, Clone, config::Configuration)]
#[section(input=bin)]
pub struct RedshiftLoader {
    host: String,
    port: u16,
    user: String,
    #[field_type(password)]
    password: String,
    database: String,
    iam_role: String,
    data_format: String,
    ignore_header: bool,
    region: String,
}

impl RedshiftLoader {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host: impl Into<String>,
        port: u16,
        user: impl Into<String>,
        password: impl Into<String>,
        database: impl Into<String>,
        iam_role: impl Into<String>,
        region: impl Into<String>,
        data_format: impl Into<String>,
        ignore_header: bool,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            user: user.into(),
            password: password.into(),
            database: database.into(),
            iam_role: iam_role.into(),
            region: region.into(),
            data_format: data_format.into(),
            ignore_header,
        }
    }
}

impl Default for RedshiftLoader {
    fn default() -> Self {
        Self::new(
            "localhost",
            5439,
            "user",
            "",
            "redshift",
            "arn:aws:iam::123456789012:role/MyExampleRole",
            "us-east-1",
            "CSV",
            true,
        )
    }
}
