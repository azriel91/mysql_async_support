use crate::{ResultSet, TypedValues};

/// [`ResultSet`] with typed values.
///
/// This is used for arbitrary queries received as input, so the types of the
/// values are under the `Value` enum.
///
/// # Note
///
/// * You must use prepared statements if you want types to be returned,
///   otherwise it is always returned as `Value::Bytes`
/// * There must be only one statement in the query -- i.e. no multiple selects.
/// * Not sure if nested select statements work.
///
/// However, I haven't managed to get MySQL to return `Value`s with proper
/// return types.
///
/// See:
///
/// * <https://github.com/go-sql-driver/mysql/issues/407#issuecomment-172583652>
/// * <https://dev.mysql.com/doc/refman/8.0/en/sql-prepared-statements.html>
pub type ResultSetTyped = ResultSet<TypedValues>;
