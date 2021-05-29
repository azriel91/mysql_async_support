use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
};

use indexmap::IndexMap;
use mysql_async::{prelude::FromRow, Column, FromRowError, Row};
use serde::{Deserialize, Serialize};

/// Represents a query result row, with all values stringified.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct StringValues(pub IndexMap<String, String>);

impl StringValues {
    /// Whether to use backslashes to escape values.
    ///
    /// This is written in the negative form to match `mysql_async`'s
    /// [parameter].
    ///
    /// [parameter]: https://docs.rs/mysql_async/0.27.1/mysql_async/enum.Value.html#method.as_sql
    const NO_ESCAPE_BACKSLASH: bool = true;
}

impl FromRow for StringValues {
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
        let values = row
            .unwrap()
            .into_iter()
            .map(|value| value.as_sql(Self::NO_ESCAPE_BACKSLASH))
            .map(|value| {
                // Remove surrounding `'` characters
                value
                    .strip_prefix('\'')
                    .and_then(|value| value.strip_suffix('\''))
                    .map(str::to_string)
                    .unwrap_or(value)
            });
        let values = column_names_lossy
            .into_iter()
            .zip(values)
            .collect::<IndexMap<String, String>>();

        Ok(StringValues(values))
    }
}

impl Deref for StringValues {
    type Target = IndexMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for StringValues {
    fn deref_mut(self: &mut StringValues) -> &mut Self::Target {
        &mut self.0
    }
}
