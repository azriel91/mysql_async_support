use std::fmt;

/// Error while using the `mysql_async_support` library.
#[derive(Debug)]
pub enum Error {
    /// Failed to initialize `mysql_async::PoolConstraints`.
    PoolConstraintsInitialize,
    /// Error occurred while disconnecting connection pool.
    MySqlPoolDisconnect(mysql_async::Error),
    /// Error while using the `ssh_jumper` crate.
    SshJumper(Box<ssh_jumper::model::Error>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::PoolConstraintsInitialize => {
                write!(f, "Failed to construct `mysql_async::PoolConstraints.`")
            }
            Self::MySqlPoolDisconnect(..) => {
                write!(f, "Failed to cleanly disconnect MySQL connection pool.")
            }
            Self::SshJumper(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::PoolConstraintsInitialize => None,
            Self::MySqlPoolDisconnect(error) => Some(error),
            Self::SshJumper(error) => error.source(),
        }
    }
}

impl From<ssh_jumper::model::Error> for Error {
    fn from(error: ssh_jumper::model::Error) -> Self {
        Self::SshJumper(Box::new(error))
    }
}
