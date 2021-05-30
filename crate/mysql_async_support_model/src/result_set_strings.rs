use crate::{ResultSet, StringValues};

/// [`ResultSet`] that represents values as [`String`]s
///
/// This is used for arbitrary queries received as input, so we convert all
/// values to [`String`]s.
pub type ResultSetStrings = ResultSet<StringValues>;
