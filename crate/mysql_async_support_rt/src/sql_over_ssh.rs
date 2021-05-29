use mysql_async::{OptsBuilder, PoolConstraints, PoolOpts};
use mysql_async_support_model::{DbSchemaCred, Error};
use ssh_jumper::{
    model::{HostAddress, HostSocketParams, JumpHostAuthParams, SshTunnelParams},
    SshJumper,
};

use crate::FnWithPool;

/// Runs SQL over an SSH connection..
#[derive(Clone)]
pub struct SqlOverSsh;

impl SqlOverSsh {
    /// Runs queries specified by the parameter through a new DB connection
    /// pool.
    ///
    /// # Parameters
    ///
    /// * `jump_host_address`: Address of the jump host.
    /// * `jump_host_auth_params`: Parameters to authenticate with the jump
    ///   host.
    /// * `db_address`: The database server address.
    /// * `db_schema_cred`: Credentials to access a database schema.
    /// * `queries`: Async function that runs queries against the database.
    ///
    /// # Note
    ///
    /// For some reason `pool.get_conn()` doesn't work at the same time, so the
    /// future cannot use `futures::join!()` to run multiple queries
    /// concurrently.
    pub async fn execute<'f, Queries>(
        &'f self,
        jump_host_address: HostAddress<'f>,
        jump_host_auth_params: JumpHostAuthParams<'f>,
        db_address: HostAddress<'f>,
        db_schema_cred: &'f DbSchemaCred<'f>,
        queries: Queries,
    ) -> Result<<Queries as FnWithPool<'_>>::Output, <Queries as FnWithPool<'_>>::Error>
    where
        Queries: FnWithPool<'f>,
    {
        let pool = self
            .db_pool_initialize(
                jump_host_address,
                jump_host_auth_params,
                db_address,
                db_schema_cred,
            )
            .await?;

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
        jump_host_address: HostAddress<'f>,
        jump_host_auth_params: JumpHostAuthParams<'f>,
        db_address: HostAddress<'f>,
        db_schema_cred: &'f DbSchemaCred<'f>,
    ) -> Result<mysql_async::Pool, Error> {
        let local_socket_addr = {
            let target_socket = HostSocketParams {
                address: db_address,
                port: 3306,
            };
            let ssh_params =
                SshTunnelParams::new(jump_host_address, jump_host_auth_params, target_socket);
            SshJumper::open_tunnel(&ssh_params).await?
        };

        let db_opts = OptsBuilder::default()
            .ip_or_hostname(local_socket_addr.ip().to_string())
            .tcp_port(local_socket_addr.port())
            .db_name(db_schema_cred.schema_name.as_deref())
            .user(Some(db_schema_cred.username.as_ref()))
            .pass(Some(db_schema_cred.password.as_ref()))
            .pool_opts(Some(PoolOpts::default().with_constraints(
                PoolConstraints::new(1, 10).ok_or(Error::PoolConstraintsInitialize)?,
            )));

        Ok(mysql_async::Pool::new(db_opts))
    }
}
