use std::fmt;

use ssh_jumper::model::HostAddress;

use crate::QueryTarget;

/// Error while using the `mysql_async_support` library.
#[derive(Debug)]
pub enum Error {
    /// Failed to initialize `mysql_async::PoolConstraints`.
    PoolConstraintsInitialize,
    /// Failed to get MySQL connection.
    MySqlConnectionRetrieve(mysql_async::Error),
    /// Failed to prepare SQL statement.
    MySqlPrepare(mysql_async::Error),
    /// Failed to execute SQL query.
    MySqlExecute(mysql_async::Error),
    /// Failed to fetch result set from query execution.
    ///
    /// One query may have multiple result sets, and we may fail to fetch a
    /// later one.
    QueryResultSetFetch(mysql_async::Error),
    /// Error occurred while disconnecting connection pool.
    MySqlPoolDisconnect(mysql_async::Error),
    /// SSH connection initialization failed.
    SshConnInit,
    /// SSH tunnel was not found for a query target.
    SshTunnelNotFound {
        /// Address of the jump host.
        jump_host_address: HostAddress<'static>,
        /// The query target that the SSH tunnel wasn't found for.
        query_target: QueryTarget<'static>,
    },
    /// Error while using the `ssh_jumper` crate.
    SshJumper(Box<ssh_jumper::model::Error>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::PoolConstraintsInitialize => {
                write!(f, "Failed to construct `mysql_async::PoolConstraints.`")
            }
            Self::MySqlConnectionRetrieve(..) => write!(f, "Failed to get MySql connection."),
            Self::MySqlPrepare(..) => write!(f, "Failed to prepare SQL statement."),
            Self::MySqlExecute(..) => write!(f, "Failed to execute SQL query."),
            Self::QueryResultSetFetch(..) => write!(f, "Failed to fetch next query result set."),
            Self::MySqlPoolDisconnect(..) => {
                write!(f, "Failed to cleanly disconnect MySQL connection pool.")
            }
            Self::SshConnInit => write!(f, "SSH connection initialization failed."),
            Self::SshTunnelNotFound {
                jump_host_address,
                query_target,
            } => write!(
                f,
                "Expected SSH tunnel for `{query_target}: {db_address}` to be established through jump host: `{jump_host}`. This is likely a bug.",
                query_target = query_target.name,
                db_address = query_target.db_address,
                jump_host = jump_host_address
            ),
            Self::SshJumper(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::PoolConstraintsInitialize => None,
            Self::MySqlConnectionRetrieve(error) => Some(error),
            Self::MySqlPrepare(error) => Some(error),
            Self::MySqlExecute(error) => Some(error),
            Self::QueryResultSetFetch(error) => Some(error),
            Self::MySqlPoolDisconnect(error) => Some(error),
            Self::SshConnInit => None,
            Self::SshTunnelNotFound { .. } => None,
            Self::SshJumper(error) => error.source(),
        }
    }
}

impl From<ssh_jumper::model::Error> for Error {
    fn from(error: ssh_jumper::model::Error) -> Self {
        Self::SshJumper(Box::new(error))
    }
}
