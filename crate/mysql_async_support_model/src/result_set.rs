use serde::{Deserialize, Serialize};

use crate::StringValues;

/// Message, warning count, and result values for a single statement.
///
/// This is used for arbitrary queries received as input, so we convert all
/// values to [`String`]s.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ResultSet {
    /// Number of rows affected by the query.
    pub affected_rows: u64,
    /// Number of warnings.
    pub warning_count: u16,
    /// Message returned by the server.
    pub info: String,
    /// Values returned by the statement.
    pub values: Vec<StringValues>,
}
