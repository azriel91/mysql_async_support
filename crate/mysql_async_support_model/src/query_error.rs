use crate::Error;

/// Query target name and error when attempting to run the query.
#[derive(Debug)]
pub struct QueryError {
    /// Name of the query target.
    pub name: String,
    /// The error that occurred when running the query.
    pub error: Error,
}

impl std::ops::Deref for QueryError {
    type Target = Error;

    fn deref(&self) -> &Self::Target {
        &self.error
    }
}
