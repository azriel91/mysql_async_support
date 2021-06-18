use serde::{Deserialize, Serialize};

use crate::ResultSet;

/// Query target name and result sets.
///
/// # Parameters
///
/// * `T`: Type of the result set. You may use [`TypedValues`] for a generic
///   implementation.
///
/// [`TypedValues`]: crate::TypedValues
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct QueryResult<T> {
    /// Name of the query target.
    pub name: String,
    /// Result sets returned by the query.
    pub result_sets: Vec<ResultSet<T>>,
}
