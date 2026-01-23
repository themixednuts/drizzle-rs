use crate::prelude::*;
use crate::{
    OwnedParam, ParamBind, SQL, SQLChunk, ToSQL,
    prepared::{PreparedStatement, bind_values_internal},
    traits::SQLParam,
};
use compact_str::CompactString;
use core::fmt;
use smallvec::SmallVec;

/// An owned version of PreparedStatement with no lifetime dependencies
#[derive(Debug, Clone)]
pub struct OwnedPreparedStatement<V: SQLParam> {
    /// Pre-rendered text segments
    pub text_segments: Box<[CompactString]>,
    /// Parameter placeholders (in order) - only placeholders, no values
    pub params: Box<[OwnedParam<V>]>,
    /// Fully rendered SQL with placeholders for this dialect
    pub sql: CompactString,
}
impl<V: SQLParam> core::fmt::Display for OwnedPreparedStatement<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.sql())
    }
}

impl<'a, V: SQLParam> From<PreparedStatement<'a, V>> for OwnedPreparedStatement<V> {
    fn from(prepared: PreparedStatement<'a, V>) -> Self {
        OwnedPreparedStatement {
            text_segments: prepared.text_segments,
            params: prepared.params.into_iter().map(|p| p.into()).collect(),
            sql: prepared.sql,
        }
    }
}

impl<V: SQLParam> OwnedPreparedStatement<V> {
    /// Bind parameters and return SQL with dialect-appropriate placeholders.
    /// Uses `$1, $2, ...` for PostgreSQL, `?` for SQLite/MySQL.
    pub fn bind<'a, T: SQLParam + Into<V>>(
        &self,
        param_binds: impl IntoIterator<Item = ParamBind<'a, T>>,
    ) -> (&str, impl Iterator<Item = V>) {
        let bound_params = bind_values_internal(
            &self.params,
            param_binds,
            |p| p.placeholder.name,
            |p| p.value.as_ref(), // OwnedParam can store values
        );

        (self.sql.as_str(), bound_params)
    }

    /// Returns the fully rendered SQL with placeholders.
    pub fn sql(&self) -> &str {
        self.sql.as_str()
    }
}

impl<'a, V: SQLParam> ToSQL<'a, V> for OwnedPreparedStatement<V> {
    fn to_sql(&self) -> SQL<'a, V> {
        // Calculate exact capacity needed: text_segments.len() + params.len()
        let capacity = self.text_segments.len() + self.params.len();
        let mut chunks = SmallVec::with_capacity(capacity);

        // Interleave text segments and params: text[0], param[0], text[1], param[1], ..., text[n]
        // Use iterators to avoid bounds checking and minimize allocations
        let mut param_iter = self.params.iter();

        for text_segment in &self.text_segments {
            chunks.push(SQLChunk::Raw(Cow::Owned(text_segment.to_string())));

            // Add corresponding param if available
            if let Some(param) = param_iter.next() {
                chunks.push(SQLChunk::Param(param.clone().into()));
            }
        }

        SQL { chunks }
    }
}
