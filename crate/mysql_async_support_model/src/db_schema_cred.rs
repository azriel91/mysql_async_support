use std::borrow::Cow;

/// Credentials to access a database schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DbSchemaCred<'db_cred> {
    /// Database schema name, `None` if no schema is selected.
    pub schema_name: Option<Cow<'db_cred, str>>,
    /// Username to login to the database server.
    pub username: Cow<'db_cred, str>,
    /// Password to login to the database server.
    pub password: Cow<'db_cred, str>,
}

impl<'db_cred> DbSchemaCred<'db_cred> {
    /// Returns an owned version of self.
    pub fn into_static(self) -> DbSchemaCred<'static> {
        let DbSchemaCred::<'db_cred> {
            schema_name,
            username,
            password,
        } = self;
        let schema_name = schema_name.map(Cow::into_owned).map(Cow::Owned);
        let username = Cow::Owned(username.into_owned());
        let password = Cow::Owned(password.into_owned());

        DbSchemaCred {
            schema_name,
            username,
            password,
        }
    }
}
