use serde::{Deserialize, Serialize};

use crate::ResultSetTyped;

/// Query target name and result sets.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct QueryResult {
    /// Name of the query target.
    pub name: String,
    /// Result sets returned by the query.
    pub result_sets: Vec<ResultSetTyped>,
}
