use crate::prelude::*;
use crate::{
    OwnedParam, ParamBind, SQL, SQLChunk, ToSQL,
    prepared::{PreparedStatement, bind_values_internal},
    traits::SQLParam,
};
use compact_str::CompactString;
use core::fmt;
use hashbrown::HashMap;
use smallvec::SmallVec;

/// A [`PreparedStatement`] that owns all its data, so it can be stored
/// without a lifetime (for example in a cache).
#[derive(Debug, Clone)]
pub struct OwnedPreparedStatement<V: SQLParam> {
    /// Rendered SQL text between the parameters; one more than `params`.
    pub text_segments: Box<[CompactString]>,
    /// The parameters, in order, with any values fixed at render time.
    pub params: Box<[OwnedParam<V>]>,
    /// The full SQL text, with the dialect's placeholders.
    pub sql: CompactString,
}
impl<V: SQLParam> core::fmt::Display for OwnedPreparedStatement<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.sql())
    }
}

impl<'a, V: SQLParam> From<PreparedStatement<'a, V>> for OwnedPreparedStatement<V> {
    fn from(prepared: PreparedStatement<'a, V>) -> Self {
        Self {
            text_segments: prepared.text_segments,
            params: prepared
                .params
                .into_iter()
                .map(core::convert::Into::into)
                .collect(),
            sql: prepared.sql,
        }
    }
}

impl<V: SQLParam> OwnedPreparedStatement<V> {
    /// Returns how many bindings [`bind`](Self::bind) expects.
    ///
    /// Counts parameters without a value, with each placeholder name counted
    /// once, since one binding fills every use of a name.
    #[must_use]
    pub fn external_param_count(&self) -> usize {
        let mut named = HashMap::<&str, ()>::new();
        let mut positional = 0usize;
        for param in &self.params {
            if param.value.is_some() {
                continue;
            }
            match param.placeholder.name {
                Some(name) if !name.is_empty() => {
                    named.entry(name).or_insert(());
                }
                _ => positional += 1,
            }
        }
        named.len() + positional
    }

    /// Binds values to the placeholders and returns the SQL text with the
    /// values to send, in order. Works like [`PreparedStatement::bind`].
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ParameterError`](crate::error::DrizzleError::ParameterError)
    /// when a name is bound twice, a placeholder has no binding, or a binding
    /// matches no placeholder.
    pub fn bind<'a, T: SQLParam + Into<V>>(
        &self,
        param_binds: impl IntoIterator<Item = ParamBind<'a, T>>,
    ) -> crate::error::Result<(&str, impl Iterator<Item = V>)> {
        let bound_params = bind_values_internal(
            &self.params,
            param_binds,
            |p| p.placeholder.name,
            |p| p.value.as_ref(), // OwnedParam can store values
        )?;

        Ok((self.sql.as_str(), bound_params.into_iter()))
    }

    /// Returns the SQL text, with the dialect's placeholders.
    #[must_use]
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
