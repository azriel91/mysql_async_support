use std::{collections::HashMap, net::SocketAddr};

use futures::stream::{self, StreamExt};
use mysql_async::{
    prelude::{FromRow, Queryable},
    BinaryProtocol,
};
use mysql_async_support_model::{Error, QueryError, QueryResult, QueryTarget, ResultSet};
use ssh_jumper::{
    model::{HostAddress, HostSocketParams, JumpHostAuthParams, SshTunnelParams},
    SshJumper,
};

use crate::{FnWithPool, SqlOverSsh, SshTunnelManager};

/// Runs queries for one or more query targets.
#[derive(Clone)]
pub struct QueryRunner {
    /// Runs SQL over an SSH connection.
    pub sql_over_ssh: SqlOverSsh,
    /// Maximum number of SSH connections to run concurrently.
    pub ssh_concurrent_limit: usize,
    /// Maximum number of tunnels per SSH connection.
    pub tunnels_per_ssh_connection: usize,
}

impl QueryRunner {
    /// Maximum number of tunnels per SSH connection.
    pub const SSH_CONCURRENT_LIMIT_DEFAULT: usize = 20;
    /// Maximum number of tunnels per SSH connection.
    pub const TUNNELS_PER_SSH_CONNECTION_DEFAULT: usize = 3;

    /// Returns a new `QueryRunner` with the given connection limits.
    pub fn new(ssh_concurrent_limit: usize, tunnels_per_ssh_connection: usize) -> Self {
        Self {
            sql_over_ssh: SqlOverSsh,
            ssh_concurrent_limit,
            tunnels_per_ssh_connection,
        }
    }

    /// Queries a database over an SSH connection.
    pub async fn query<T>(
        &self,
        jump_host_address: &HostAddress<'_>,
        jump_host_auth_params: &JumpHostAuthParams<'_>,
        query_target: &QueryTarget<'_>,
        sql_text: &str,
    ) -> Result<QueryResult<T>, Error>
    where
        T: FromRow + Send + 'static,
    {
        let db_tunnel = {
            let jump_host_address = jump_host_address.clone();
            let jump_host_auth_params = jump_host_auth_params.clone();
            let target_socket = HostSocketParams {
                address: query_target.db_address.clone(),
                port: 3306,
            };
            let ssh_params =
                SshTunnelParams::new(jump_host_address, jump_host_auth_params, target_socket);
            SshJumper::open_tunnel(&ssh_params).await?
        };

        self.sql_over_ssh
            .exec(
                db_tunnel,
                query_target.db_schema_cred.clone(),
                |pool: mysql_async::Pool| async {
                    let result = Self::query_run(&pool, query_target, sql_text).await;
                    (pool, result)
                },
            )
            .await
    }

    /// Queries multiple query targets with the same query.
    pub async fn query_multi<T>(
        &self,
        jump_host_address: &HostAddress<'_>,
        jump_host_auth_params: &JumpHostAuthParams<'_>,
        query_targets: &[QueryTarget<'_>],
        sql_text: &str,
    ) -> (Vec<QueryResult<T>>, Vec<QueryError>)
    where
        T: FromRow + Send + 'static,
    {
        stream::iter(query_targets.chunks(self.tunnels_per_ssh_connection))
            .then(|query_targets_chunk| async move {
                let ssh_connections = SshTunnelManager::prepare_tunnels(
                    jump_host_address,
                    jump_host_auth_params,
                    query_targets_chunk,
                )
                .await;

                async move {
                    match ssh_connections {
                        Ok((_ssh_sessions, qt_name_to_tunnel)) => {
                            self.query_over_tunnels(
                                jump_host_address,
                                query_targets_chunk,
                                sql_text,
                                qt_name_to_tunnel,
                            )
                            .await
                        }
                        Err(e) => {
                            let mut error = Some(e);
                            let errors = query_targets_chunk
                                .iter()
                                .map(|query_target| QueryError {
                                    name: query_target.name.clone().into_owned(),
                                    error: error.take().unwrap_or(Error::SshConnInit),
                                })
                                .collect::<Vec<QueryError>>();
                            (vec![], errors)
                        }
                    }
                }
            })
            .buffered(self.ssh_concurrent_limit)
            .fold(
                (
                    Vec::with_capacity(query_targets.len()),
                    Vec::with_capacity(query_targets.len()),
                ),
                |(mut query_results, mut query_errors),
                 (query_results_chunk, query_errors_chunk)| async move {
                    query_results.extend(query_results_chunk);
                    query_errors.extend(query_errors_chunk);

                    (query_results, query_errors)
                },
            )
            .await
    }

    /// Queries multiple query targets with the same query.
    pub async fn exec_multi<'f, Queries>(
        &'f self,
        jump_host_address: HostAddress<'f>,
        jump_host_auth_params: JumpHostAuthParams<'f>,
        query_targets: &'f [QueryTarget<'f>],
        queries: Queries,
    ) -> (
        Vec<(&'f QueryTarget<'f>, <Queries as FnWithPool<'f>>::Output)>,
        Vec<(&'f QueryTarget<'f>, <Queries as FnWithPool<'f>>::Error)>,
    )
    where
        Queries: FnWithPool<'f> + Copy,
        <Queries as FnWithPool<'f>>::Error: From<Error>,
    {
        let jump_host_address = &jump_host_address;
        let jump_host_auth_params = &jump_host_auth_params;
        stream::iter(query_targets.chunks(self.tunnels_per_ssh_connection))
            .then(|query_targets_chunk| async move {
                let ssh_connections = SshTunnelManager::prepare_tunnels(
                    jump_host_address,
                    jump_host_auth_params,
                    query_targets_chunk,
                )
                .await;

                async move {
                    match ssh_connections {
                        Ok((_ssh_sessions, qt_name_to_tunnel)) => {
                            self
                                .exec_over_tunnels(jump_host_address, query_targets, queries, qt_name_to_tunnel)
                                .await
                        }
                        Err(e) => {
                            let mut error = Some(e);
                            let errors = query_targets_chunk
                                .iter()
                                .map(|query_target| {

                                    let error = error.take().unwrap_or(Error::SshConnInit);
                                    (query_target, <Queries as FnWithPool<'f>>::Error::from(error))
                                })
                                .collect::<Vec<(&QueryTarget<'_>, <Queries as FnWithPool<'f>>::Error)>>();
                            (vec![], errors)
                        }
                    }
                }
            })
            .buffered(self.ssh_concurrent_limit)
            .fold(
                (
                    Vec::with_capacity(query_targets.len()),
                    Vec::with_capacity(query_targets.len()),
                ),
                |(mut exec_results, mut exec_errors),
                 (exec_results_chunk, exec_errors_chunk)| async move {
                    exec_results.extend(exec_results_chunk);
                    exec_errors.extend(exec_errors_chunk);

                    (exec_results, exec_errors)
                },
            )
            .await
    }

    async fn query_over_tunnels<T>(
        &self,
        jump_host_address: &HostAddress<'_>,
        query_targets: &[QueryTarget<'_>],
        sql_text: &str,
        qt_name_to_tunnel: HashMap<&str, SocketAddr>,
    ) -> (Vec<QueryResult<T>>, Vec<QueryError>)
    where
        T: FromRow + Send + 'static,
    {
        let qt_name_to_tunnel = &qt_name_to_tunnel;
        let query_results_and_errors = stream::iter(query_targets.iter())
            .map(|query_target| async move {
                let db_tunnel = *qt_name_to_tunnel
                    .get(query_target.name.as_ref())
                    .ok_or_else(|| {
                        let error = Error::SshTunnelNotFound {
                            jump_host_address: jump_host_address.into_static(),
                            query_target: query_target.clone().into_static(),
                        };
                        QueryError {
                            name: query_target.name.to_string(),
                            error,
                        }
                    })?;
                self.sql_over_ssh
                    .exec(
                        db_tunnel,
                        query_target.db_schema_cred.clone(),
                        |pool: mysql_async::Pool| async {
                            let result = Self::query_run(&pool, query_target, sql_text).await;
                            (pool, result)
                        },
                    )
                    .await
                    .map_err(|error| QueryError {
                        name: query_target.name.to_string(),
                        error,
                    })
            })
            .buffered(self.tunnels_per_ssh_connection)
            .fold(
                (Vec::new(), Vec::new()),
                |(mut query_results, mut query_errors), result| async {
                    match result {
                        Ok(query_result) => query_results.push(query_result),
                        Err(query_error) => query_errors.push(query_error),
                    }
                    (query_results, query_errors)
                },
            )
            .await;

        query_results_and_errors
    }

    async fn query_run<T>(
        pool: &mysql_async::Pool,
        query_target: &QueryTarget<'_>,
        sql_text: &str,
    ) -> Result<QueryResult<T>, Error>
    where
        T: FromRow + Send + 'static,
    {
        match pool.get_conn().await {
            Ok(mut conn) => {
                let statement = conn.prep(sql_text).await.map_err(Error::MySqlPrepare);
                match statement {
                    Ok(statement) => {
                        let result = conn
                            .exec_iter(statement, ())
                            .await
                            .map_err(Error::MySqlExecute);

                        match result {
                            Ok(mut query_result) => {
                                Self::query_result_fetch::<T>(
                                    query_target.name.to_string(),
                                    &mut query_result,
                                )
                                .await
                            }
                            Err(e) => Err(e),
                        }
                    }
                    Err(e) => Err(e),
                }
            }
            Err(e) => Err(Error::MySqlConnectionRetrieve(e)),
        }
    }

    async fn query_result_fetch<T>(
        query_target_name: String,
        query_result: &mut mysql_async::QueryResult<'_, '_, BinaryProtocol>,
    ) -> Result<QueryResult<T>, Error>
    where
        T: FromRow + Send + 'static,
    {
        // A query result may have multiple result sets.
        // Each set may have a different number of columns.
        //
        // `Stream` is not implemented for `QueryResult`, so we use imperative style.
        // See the following issue for reasons:
        // https://github.com/blackbeam/mysql_async/issues/90
        let mut result_sets = Vec::new();
        while !query_result.is_empty() {
            let values = query_result
                .collect::<T>()
                .await
                .map_err(Error::QueryResultSetFetch)?;
            let affected_rows = query_result.affected_rows();
            let warning_count = query_result.warnings();
            let info = query_result.info().into_owned();

            let result_set = ResultSet::<T> {
                affected_rows,
                info,
                warning_count,
                values,
            };
            result_sets.push(result_set);
        }

        Ok(QueryResult {
            name: query_target_name,
            result_sets,
        })
    }

    async fn exec_over_tunnels<'f, Queries>(
        &'f self,
        jump_host_address: &HostAddress<'f>,
        query_targets: &'f [QueryTarget<'f>],
        queries: Queries,
        qt_name_to_tunnel: HashMap<&'f str, SocketAddr>,
    ) -> (
        Vec<(&'f QueryTarget<'f>, <Queries as FnWithPool<'f>>::Output)>,
        Vec<(&'f QueryTarget<'f>, <Queries as FnWithPool<'f>>::Error)>,
    )
    where
        Queries: FnWithPool<'f> + Copy,
        <Queries as FnWithPool<'f>>::Error: From<Error>,
    {
        let qt_name_to_tunnel = &qt_name_to_tunnel;
        stream::iter(query_targets.iter())
            .map(|query_target| async move {
                let db_tunnel = *qt_name_to_tunnel
                    .get(query_target.name.as_ref())
                    .ok_or_else(|| Error::SshTunnelNotFound {
                        jump_host_address: jump_host_address.clone().into_static(),
                        query_target: query_target.clone().into_static(),
                    })
                    .map_err(<Queries as FnWithPool<'f>>::Error::from)
                    .map_err(|exec_error| (query_target, exec_error))?;

                self.sql_over_ssh
                    .exec(db_tunnel, query_target.db_schema_cred.clone(), queries)
                    .await
                    .map(|exec_result| (query_target, exec_result))
                    .map_err(|exec_error| (query_target, exec_error))
            })
            .buffered(self.tunnels_per_ssh_connection)
            .fold(
                (Vec::new(), Vec::new()),
                |(mut exec_results, mut exec_errors), result| async {
                    match result {
                        Ok(exec_result) => exec_results.push(exec_result),
                        Err(exec_error) => exec_errors.push(exec_error),
                    }
                    (exec_results, exec_errors)
                },
            )
            .await
    }
}

impl Default for QueryRunner {
    fn default() -> Self {
        Self::new(
            Self::SSH_CONCURRENT_LIMIT_DEFAULT,
            Self::TUNNELS_PER_SSH_CONNECTION_DEFAULT,
        )
    }
}
