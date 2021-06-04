use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
};

use indexmap::IndexMap;
use mysql_async::{prelude::FromRow, Column, FromRowError, Row};
use serde::{Deserialize, Serialize};

use crate::Value;

/// Represents a query result row, with all values stringified.
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
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct TypedValues(pub IndexMap<String, Value>);

impl FromRow for TypedValues {
    // `column_names_lossy` appears to be a needless_collect to clippy, but it is
    // needed as `row.unwrap()` consumes `Row`.
    #[allow(clippy::needless_collect)]
    fn from_row_opt(row: Row) -> Result<Self, FromRowError> {
        let column_names_lossy = row
            .columns_ref()
            .iter()
            .map(Column::name_str)
            .map(Cow::into_owned)
            .collect::<Vec<String>>();

        // unwrap here is not `Result::unwrap`, but `Row::unwrap`
        let values = row.unwrap().into_iter().map(Value::from);

        let values = column_names_lossy
            .into_iter()
            .zip(values)
            .collect::<IndexMap<String, Value>>();

        Ok(TypedValues(values))
    }
}

impl Deref for TypedValues {
    type Target = IndexMap<String, Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TypedValues {
    fn deref_mut(self: &mut TypedValues) -> &mut Self::Target {
        &mut self.0
    }
}
