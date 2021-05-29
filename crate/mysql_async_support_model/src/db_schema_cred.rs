use std::borrow::Cow;

/// Credentials to access a database schema.
#[derive(Debug)]
pub struct DbSchemaCred<'db_cred> {
    /// Database schema name, `None` if no schema is selected.
    pub schema_name: Option<Cow<'db_cred, str>>,
    /// Username to login to the database server.
    pub username: Cow<'db_cred, str>,
    /// Password to login to the database server.
    pub password: Cow<'db_cred, str>,
}
