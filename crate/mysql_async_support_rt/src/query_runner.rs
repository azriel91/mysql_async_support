use futures::stream::{self, StreamExt};
use mysql_async::{prelude::Queryable, BinaryProtocol};
use mysql_async_support_model::{
    Error, QueryError, QueryResult, QueryTarget, ResultSetTyped, TypedValues,
};
use ssh_jumper::model::{HostAddress, JumpHostAuthParams};

use crate::SqlOverSsh;

/// Runs queries for one or more query targets.
pub struct QueryRunner {
    /// Runs SQL over an SSH connection.
    pub sql_over_ssh: SqlOverSsh,
    /// Maximum number of SSH connections to run concurrently.
    pub ssh_concurrent_limit: usize,
}

impl QueryRunner {
    /// Queries multiple query targets with the same query.
    pub async fn query_multi(
        &self,
        jump_host_address: &HostAddress<'_>,
        jump_host_auth_params: &JumpHostAuthParams<'_>,
        query_targets: &[QueryTarget<'_>],
        sql_text: &str,
    ) -> Result<(Vec<QueryResult>, Vec<QueryError>), Error> {
        let query_results_and_errors = stream::iter(query_targets.iter())
            .map(|query_target| async move {
                self.sql_over_ssh
                    .execute(
                        jump_host_address.clone(),
                        jump_host_auth_params.clone(),
                        query_target.db_address.clone(),
                        query_target.db_schema_cred.clone(),
                        |pool: mysql_async::Pool| async {
                            let result = match pool.get_conn().await {
                                Ok(mut conn) => {
                                    let statement =
                                        conn.prep(sql_text).await.map_err(Error::MySqlPrepare);
                                    match statement {
                                        Ok(statement) => {
                                            let result = conn
                                                .exec_iter(statement, ())
                                                .await
                                                .map_err(Error::MySqlExecute);

                                            match result {
                                                Ok(mut query_result) => {
                                                    Self::query_result_fetch(
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
                            };

                            (pool, result)
                        },
                    )
                    .await
                    .map_err(|error| QueryError {
                        name: query_target.name.to_string(),
                        error,
                    })
            })
            .buffered(self.ssh_concurrent_limit)
            .fold(
                (Vec::new(), Vec::new()),
                |(mut website_query_results, mut website_query_errors), result| async {
                    match result {
                        Ok(website_query_result) => {
                            website_query_results.push(website_query_result)
                        }
                        Err(website_query_error) => website_query_errors.push(website_query_error),
                    }
                    (website_query_results, website_query_errors)
                },
            )
            .await;

        Ok(query_results_and_errors)
    }

    async fn query_result_fetch(
        query_target_name: String,
        query_result: &mut mysql_async::QueryResult<'_, '_, BinaryProtocol>,
    ) -> Result<QueryResult, Error> {
        // A query result may have multiple result sets.
        // Each set may have a different number of columns.
        //
        // `Stream` is not implemented for `QueryResult`, so we use imperative style.
        // See the following issue for reasons:
        // https://github.com/blackbeam/mysql_async/issues/90
        let mut result_sets = Vec::new();
        while !query_result.is_empty() {
            let values = query_result
                .collect::<TypedValues>()
                .await
                .map_err(Error::QueryResultSetFetch)?;
            let affected_rows = query_result.affected_rows();
            let warning_count = query_result.warnings();
            let info = query_result.info().into_owned();

            let result_set = ResultSetTyped {
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
}
