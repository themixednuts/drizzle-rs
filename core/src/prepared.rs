pub mod owned;
use crate::{
    Param, ParamBind, SQL, SQLChunk, ToSQL, prepared::owned::OwnedPreparedStatement,
    traits::SQLParam,
};
use compact_str::CompactString;
use smallvec::SmallVec;
use std::{borrow::Cow, collections::HashMap, fmt};

/// A pre-rendered SQL statement with parameter placeholders
/// Structure: [text, param, text, param, text] where text segments
/// are pre-rendered and params are placeholders to be bound later
#[derive(Debug, Clone)]
pub struct PreparedStatement<'a, V: SQLParam> {
    /// Pre-rendered text segments
    pub text_segments: Box<[CompactString]>,
    /// Parameter placeholders (in order)  
    pub params: Box<[Param<'a, V>]>,
}

impl<'a, V: SQLParam> From<OwnedPreparedStatement<V>> for PreparedStatement<'a, V> {
    fn from(value: OwnedPreparedStatement<V>) -> Self {
        Self {
            text_segments: value.text_segments,
            params: value.params.iter().map(|v| v.clone().into()).collect(),
        }
    }
}

impl<'a, V: SQLParam + std::fmt::Display> std::fmt::Display for PreparedStatement<'a, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_sql())
    }
}

/// Internal helper for binding parameters with optimizations
/// Returns SQL string and an iterator over bound parameter values
pub(crate) fn bind_parameters_internal<'a, V, T, P>(
    text_segments: &[CompactString],
    params: &[P],
    param_binds: impl IntoIterator<Item = ParamBind<'a, T>>,
    param_name_fn: impl Fn(&P) -> Option<&str>,
    param_value_fn: impl Fn(&P) -> Option<&V>,
    placeholder_fn: impl Fn(&P) -> String,
) -> (String, impl Iterator<Item = V>)
where
    V: SQLParam + Clone,
    T: SQLParam + Into<V>,
{
    // Collect param binds into HashMap for efficient lookup
    let param_map: HashMap<&str, V> = param_binds
        .into_iter()
        .map(|p| (p.name, p.value.into()))
        .collect();

    // Pre-allocate string capacity based on text segments total length
    let estimated_capacity: usize =
        text_segments.iter().map(|s| s.len()).sum::<usize>() + params.len() * 8;
    let mut sql = String::with_capacity(estimated_capacity);
    let mut bound_params = SmallVec::<[V; 8]>::new_const();

    // Use iterator to avoid bounds checking
    let mut param_iter = params.iter();

    for text_segment in text_segments {
        sql.push_str(text_segment);

        if let Some(param) = param_iter.next() {
            if let Some(name) = param_name_fn(param) {
                // Named parameter
                if let Some(value) = param_map.get(name) {
                    bound_params.push(value.clone());
                    sql.push_str(&placeholder_fn(param));
                } else {
                    // Parameter not found, keep placeholder
                    sql.push_str(&placeholder_fn(param));
                }
            } else {
                // Positional parameter - use existing value if any
                if let Some(value) = param_value_fn(param) {
                    bound_params.push(value.clone());
                    sql.push_str(&placeholder_fn(param));
                }
            }
        }
    }

    (sql, bound_params.into_iter())
}

impl<'a, V: SQLParam> PreparedStatement<'a, V> {
    /// Bind parameters and render final SQL string
    pub fn bind<T: SQLParam + Into<V>>(
        &self,
        param_binds: impl IntoIterator<Item = ParamBind<'a, T>>,
    ) -> (String, impl Iterator<Item = V>) {
        bind_parameters_internal(
            &self.text_segments,
            &self.params,
            param_binds,
            |p| p.placeholder.name,
            |p| p.value.as_ref().map(|v| v.as_ref()),
            |p| p.placeholder.to_string(),
        )
    }
}

impl<'a, V: SQLParam> ToSQL<'a, V> for PreparedStatement<'a, V> {
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
                chunks.push(SQLChunk::Param(param.clone()));
            }
        }

        SQL { chunks }
    }
}
/// Pre-render SQL by processing chunks and separating text from parameters
/// This preserves the original SQL pattern detection logic while creating a PreparedSQL
/// that can be efficiently bound and executed multiple times
pub fn prepare_render<'a, V: SQLParam>(sql: SQL<'a, V>) -> PreparedStatement<'a, V> {
    let mut text_segments = Vec::new();
    let mut params = Vec::new();
    let mut current_text = CompactString::default();

    // Process chunks with original pattern detection logic preserved
    for (i, chunk) in sql.chunks.iter().enumerate() {
        match chunk {
            SQLChunk::Param(param) => {
                // End current text segment and start a new one
                text_segments.push(current_text);
                current_text = CompactString::default();
                params.push(param.clone());
            }
            SQLChunk::Text(text) if text.is_empty() => {
                // Handle empty text - check for SELECT-FROM-TABLE pattern
                if let Some(table) = sql.detect_pattern_at(i) {
                    sql.write_qualified_columns(&mut current_text, table);
                }
            }
            SQLChunk::Text(text) if text.trim().eq_ignore_ascii_case("SELECT") => {
                // Check if this is a SELECT-FROM-TABLE pattern (SELECT with no columns)
                if let Some(table) = sql.detect_select_from_table_pattern(i) {
                    current_text.push_str("SELECT ");
                    sql.write_qualified_columns(&mut current_text, table);
                } else {
                    current_text.push_str(text);
                }
            }
            SQLChunk::Text(text) => {
                current_text.push_str(text);
            }
            SQLChunk::Table(table) => {
                current_text.push('"');
                current_text.push_str(table.name());
                current_text.push('"');
            }
            SQLChunk::Column(column) => {
                current_text.push('"');
                current_text.push_str(column.table().name());
                current_text.push_str(r#"".""#);
                current_text.push_str(column.name());
                current_text.push('"');
            }
            SQLChunk::Alias { chunk, alias } => {
                // Process the nested chunk first
                sql.write_chunk(&mut current_text, chunk, i);
                current_text.push_str(" AS ");
                current_text.push_str(alias);
            }
            SQLChunk::Subquery(sql) => {
                // Process subquery like nested SQL but with parentheses
                current_text.push('(');
                let nested_prepared = prepare_render(sql.as_ref().clone());

                // Merge the nested prepared SQL into current one
                for (j, text_segment) in nested_prepared.text_segments.iter().enumerate() {
                    if j == 0 {
                        // First segment goes into current text
                        current_text.push_str(text_segment);
                    } else {
                        // Subsequent segments create new text segments with params between
                        if let Some(param) = nested_prepared.params.get(j - 1) {
                            text_segments.push(current_text);
                            current_text = CompactString::default();
                            params.push(param.clone());
                        }
                        current_text.push_str(text_segment);
                    }
                }

                // Add any remaining parameters from the nested prepared
                for remaining_param in nested_prepared
                    .params
                    .iter()
                    .skip(nested_prepared.text_segments.len() - 1)
                {
                    text_segments.push(current_text);
                    current_text = CompactString::default();
                    params.push(remaining_param.clone());
                }

                current_text.push(')');
            }
            SQLChunk::SQL(nested_sql) => {
                // Recursively process nested SQL
                let nested_prepared = prepare_render(nested_sql.as_ref().clone());

                // Merge the nested prepared SQL into current one
                for (j, text_segment) in nested_prepared.text_segments.iter().enumerate() {
                    if j == 0 {
                        // First segment goes into current text
                        current_text.push_str(text_segment);
                    } else {
                        // Subsequent segments create new text segments with params between
                        if let Some(param) = nested_prepared.params.get(j - 1) {
                            text_segments.push(current_text);
                            current_text = CompactString::default();
                            params.push(param.clone());
                        }
                        current_text.push_str(text_segment);
                    }
                }
            }
        }

        // Add spacing between chunks if needed
        if sql.needs_space(i) {
            current_text.push(' ');
        }
    }

    // Don't forget the final text segment
    text_segments.push(current_text);

    PreparedStatement {
        text_segments: text_segments.into_boxed_slice(),
        params: params.into_boxed_slice(),
    }
}
