use crate::{ResultSet, TypedValues};

/// [`ResultSet`] with typed values.
///
/// This is used for arbitrary queries received as input, so the types of the
/// values are under the `Value` enum.
pub type ResultSetTyped = ResultSet<TypedValues>;
