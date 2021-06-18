use std::net::SocketAddr;

use mysql_async::{OptsBuilder, PoolConstraints, PoolOpts};
use mysql_async_support_model::{DbSchemaCred, Error};

use crate::FnWithPool;

/// Runs SQL over an SSH connection.
#[derive(Clone)]
pub struct SqlOverSsh;

impl SqlOverSsh {
    /// Runs queries specified by the parameter through a new DB connection
    /// pool.
    ///
    /// # Parameters
    ///
    /// * `db_address`: Address to connect to the database server.
    /// * `db_schema_cred`: Credentials to access a database schema.
    /// * `queries`: Async function that runs queries against the database.
    ///
    /// # Note
    ///
    /// For some reason `pool.get_conn()` doesn't work at the same time, so the
    /// future cannot use `futures::join!()` to run multiple queries
    /// concurrently.
    pub async fn exec<'f, Queries>(
        &'f self,
        db_address: SocketAddr,
        db_schema_cred: DbSchemaCred<'f>,
        queries: Queries,
    ) -> Result<<Queries as FnWithPool<'_>>::Output, <Queries as FnWithPool<'_>>::Error>
    where
        Queries: FnWithPool<'f>,
    {
        let pool = self.db_pool_initialize(db_address, db_schema_cred).await?;

        // Ideally we should be able to pass in `&mysql_async::Pool`, but from consumer
        // code, Rust cannot consolidate the lifetime references. See:
        //
        // * https://stackoverflow.com/questions/63517250/specify-rust-closures-lifetime
        // * https://github.com/rust-lang/rust/issues/70263
        // * https://github.com/rust-lang/rust/issues/81326
        let (pool, result) = queries.call(pool).await;

        // Pool must be disconnected explicitly because it's an asynchronous operation.
        pool.disconnect()
            .await
            .map_err(Error::MySqlPoolDisconnect)?;

        let data = result?;

        Ok(data)
    }

    async fn db_pool_initialize<'f>(
        &self,
        db_address: SocketAddr,
        db_schema_cred: DbSchemaCred<'f>,
    ) -> Result<mysql_async::Pool, Error> {
        let db_opts = OptsBuilder::default()
            .ip_or_hostname(db_address.ip().to_string())
            .tcp_port(db_address.port())
            .db_name(db_schema_cred.schema_name.as_deref())
            .user(Some(db_schema_cred.username.as_ref()))
            .pass(Some(db_schema_cred.password.as_ref()))
            .pool_opts(Some(PoolOpts::default().with_constraints(
                PoolConstraints::new(1, 2).ok_or(Error::PoolConstraintsInitialize)?,
            )));

        Ok(mysql_async::Pool::new(db_opts))
    }
}
