use std::{borrow::Cow, fmt};

use compact_str::CompactString;
use smallvec::SmallVec;

use crate::{
    OwnedParam, ParamBind, SQL, SQLChunk, ToSQL, prepared::PreparedStatement, traits::SQLParam,
};

/// An owned version of PreparedStatement with no lifetime dependencies
#[derive(Debug, Clone)]
pub struct OwnedPreparedStatement<V: SQLParam> {
    /// Pre-rendered text segments
    pub text_segments: Box<[CompactString]>,
    /// Parameter placeholders (in order) - only placeholders, no values
    pub params: Box<[OwnedParam<V>]>,
}
impl<V: SQLParam + std::fmt::Display + 'static> std::fmt::Display for OwnedPreparedStatement<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_sql())
    }
}

impl<'a, V: SQLParam> From<PreparedStatement<'a, V>> for OwnedPreparedStatement<V> {
    fn from(prepared: PreparedStatement<'a, V>) -> Self {
        OwnedPreparedStatement {
            text_segments: prepared.text_segments,
            params: prepared.params.into_iter().map(|p| p.into()).collect(),
        }
    }
}

impl<V: SQLParam> OwnedPreparedStatement<V> {
    /// Bind parameters and render final SQL string
    pub fn bind<'a>(
        self,
        param_binds: impl IntoIterator<Item = ParamBind<'a, V>>,
    ) -> (String, Vec<V>) {
        let param_binds: Vec<_> = param_binds.into_iter().collect();
        let mut bound_params = Vec::new();
        let mut sql = String::new();

        // Create a map for quick lookup of parameter bindings by name
        let param_map: std::collections::HashMap<_, _> = param_binds
            .iter()
            .map(|bind| (bind.name, &bind.value))
            .collect();

        let mut param_iter = self.params.iter();

        for (_i, text_segment) in self.text_segments.iter().enumerate() {
            sql.push_str(text_segment);

            // Add parameter placeholder if we have one
            if let Some(owned_param) = param_iter.next() {
                if let Some(name) = &owned_param.placeholder.name {
                    // Try to find a matching parameter binding
                    if let Some(value) = param_map.get(name) {
                        bound_params.push((*value).clone());
                        sql.push_str(&format!(":{}", name));
                    } else {
                        // No binding found, use placeholder name
                        sql.push_str(&format!(":{}", name));
                    }
                } else {
                    // Positional parameter
                    sql.push('?');
                    if let Some(bind) = param_binds.get(bound_params.len()) {
                        bound_params.push(bind.value.clone());
                    }
                }
            }
        }

        (sql, bound_params)
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
            chunks.push(SQLChunk::Text(Cow::Owned(text_segment.clone())));

            // Add corresponding param if available
            if let Some(param) = param_iter.next() {
                chunks.push(SQLChunk::Param(param.clone().into()));
            }
        }

        SQL { chunks }
    }
}
