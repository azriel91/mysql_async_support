use ssh_jumper::model::HostAddress;

use crate::DbSchemaCred;

/// An entity for which to run a query.
#[derive(Debug)]
pub struct QueryTarget<'query> {
    /// Name of the query target.
    ///
    /// This is used to disambiguate targets when running the same query.
    pub name: &'query str,
    /// Address of the database server.
    pub db_address: HostAddress<'query>,
    /// DB Schema and credentials of the database.
    pub db_schema_cred: DbSchemaCred<'query>,
}
