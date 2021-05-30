use serde::{Deserialize, Serialize};

/// Message, warning count, and result values for a single statement.
///
/// See [`ResultSetStrings`] and [`ResultSetTyped`] for aliased versions of this
/// class.
///
/// # Type Parameters
///
/// * `T`: Type that represents the values from each row. This may be either
///   [`StringValues`] or [`TypedValues`].
///
/// [`StringValues`]: crate::StringValues
/// [`TypedValues`]: crate::TypedValues
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ResultSet<T> {
    /// Number of rows affected by the query.
    pub affected_rows: u64,
    /// Number of warnings.
    pub warning_count: u16,
    /// Message returned by the server.
    pub info: String,
    /// Values returned by the statement.
    pub values: Vec<T>,
}
