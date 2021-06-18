use std::borrow::Cow;

use ssh_jumper::model::HostAddress;

use crate::DbSchemaCred;

/// An entity for which to run a query.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryTarget<'query> {
    /// Name of the query target.
    ///
    /// This is used to disambiguate targets when running the same query.
    pub name: Cow<'query, str>,
    /// Address of the database server.
    pub db_address: HostAddress<'query>,
    /// DB Schema and credentials of the database.
    pub db_schema_cred: DbSchemaCred<'query>,
}

impl<'query> QueryTarget<'query> {
    /// Returns an owned version of self.
    pub fn into_static(self) -> QueryTarget<'static> {
        let QueryTarget::<'query> {
            name,
            db_address,
            db_schema_cred,
        } = self;
        let name = Cow::Owned(name.into_owned());
        let db_address = db_address.into_static();
        let db_schema_cred = db_schema_cred.into_static();

        QueryTarget {
            name,
            db_address,
            db_schema_cred,
        }
    }
}
