mod owned;
pub use owned::OwnedPreparedStatement;

use crate::prelude::*;
use crate::{
    dialect::DialectExt,
    param::{Param, ParamBind},
    sql::{SQL, SQLChunk},
    traits::{SQLParam, ToSQL},
};
use compact_str::CompactString;
use core::fmt;
use smallvec::SmallVec;

/// A pre-rendered SQL statement with parameter placeholders
/// Structure: [text, param, text, param, text] where text segments
/// are pre-rendered and params are placeholders to be bound later
#[derive(Debug, Clone)]
pub struct PreparedStatement<'a, V: SQLParam> {
    /// Pre-rendered text segments
    pub text_segments: Box<[CompactString]>,
    /// Parameter placeholders (in order)
    pub params: Box<[Param<'a, V>]>,
    /// Fully rendered SQL with placeholders for this dialect
    pub sql: CompactString,
}

impl<'a, V: SQLParam> From<OwnedPreparedStatement<V>> for PreparedStatement<'a, V> {
    fn from(value: OwnedPreparedStatement<V>) -> Self {
        Self {
            text_segments: value.text_segments,
            params: value.params.iter().map(|v| v.clone().into()).collect(),
            sql: value.sql,
        }
    }
}

impl<'a, V: SQLParam> core::fmt::Display for PreparedStatement<'a, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.sql())
    }
}

/// Internal helper for binding parameters with optimizations
/// Returns the bound parameter values in order.
pub(crate) fn bind_values_internal<'a, V, T, P>(
    params: &[P],
    param_binds: impl IntoIterator<Item = ParamBind<'a, T>>,
    param_name_fn: impl Fn(&P) -> Option<&str>,
    param_value_fn: impl Fn(&P) -> Option<&V>,
) -> impl Iterator<Item = V>
where
    V: SQLParam + Clone,
    T: SQLParam + Into<V>,
{
    // Collect param binds into name map and positional list for efficient lookup
    let mut param_map: HashMap<&str, V> = HashMap::new();
    let mut positional_params: SmallVec<[V; 8]> = SmallVec::new_const();
    for bind in param_binds {
        if bind.name.is_empty() {
            positional_params.push(bind.value.into());
        } else {
            param_map.insert(bind.name, bind.value.into());
        }
    }
    let mut positional_iter = positional_params.into_iter();

    let mut bound_params = SmallVec::<[V; 8]>::new_const();

    for param in params {
        // For parameters, prioritize internal values first, then external bindings
        if let Some(value) = param_value_fn(param) {
            // Use internal parameter value (from prepared statement)
            bound_params.push(value.clone());
        } else if let Some(name) = param_name_fn(param) {
            // If no internal value, try external binding for named parameters
            if !name.is_empty() {
                if let Some(value) = param_map.get(name) {
                    bound_params.push(value.clone());
                }
            } else if let Some(value) = positional_iter.next() {
                bound_params.push(value);
            }
        } else if let Some(value) = positional_iter.next() {
            bound_params.push(value);
        }
    }

    // Note: We don't add unmatched external parameters as they should only be used
    // for parameters that have corresponding placeholders in the SQL

    bound_params.into_iter()
}

fn render_sql_with_placeholders<P>(
    text_segments: &[CompactString],
    params: &[P],
    placeholder_fn: impl Fn(&P, usize) -> Cow<'static, str>,
) -> CompactString {
    // Pre-allocate string capacity based on text segments total length
    let estimated_capacity: usize =
        text_segments.iter().map(|s| s.len()).sum::<usize>() + params.len() * 8;
    let mut sql = String::with_capacity(estimated_capacity);

    // Use iterator to avoid bounds checking
    let mut param_iter = params.iter().enumerate();

    for text_segment in text_segments {
        sql.push_str(text_segment);

        if let Some((idx, param)) = param_iter.next() {
            // Always add the placeholder (idx + 1 for 1-based indexing)
            sql.push_str(&placeholder_fn(param, idx + 1));
        }
    }

    CompactString::new(sql)
}

impl<'a, V: SQLParam> PreparedStatement<'a, V> {
    /// Bind parameters and return SQL with dialect-appropriate placeholders.
    /// Uses `$1, $2, ...` for PostgreSQL, `:name` or `?` for SQLite, `?` for MySQL.
    pub fn bind<T: SQLParam + Into<V>>(
        &self,
        param_binds: impl IntoIterator<Item = ParamBind<'a, T>>,
    ) -> (&str, impl Iterator<Item = V>) {
        let bound_params = bind_values_internal(
            &self.params,
            param_binds,
            |p| p.placeholder.name,
            |p| p.value.as_ref().map(|v| v.as_ref()),
        );

        (self.sql.as_str(), bound_params)
    }

    /// Returns the fully rendered SQL with placeholders.
    pub fn sql(&self) -> &str {
        self.sql.as_str()
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
            chunks.push(SQLChunk::Raw(Cow::Owned(text_segment.to_string())));

            // Add corresponding param if available
            if let Some(param) = param_iter.next() {
                chunks.push(SQLChunk::Param(param.clone()));
            }
        }

        SQL { chunks }
    }
}
/// Pre-render SQL by processing chunks and separating text from parameters
pub fn prepare_render<'a, V: SQLParam>(sql: SQL<'a, V>) -> PreparedStatement<'a, V> {
    use crate::dialect::Dialect;

    let mut text_segments = Vec::new();
    let mut params = Vec::new();
    let mut current_text = String::new();

    for (i, chunk) in sql.chunks.iter().enumerate() {
        match chunk {
            SQLChunk::Param(param) => {
                text_segments.push(CompactString::new(&current_text));
                current_text.clear();
                params.push(param.clone());
            }
            _ => {
                sql.write_chunk_to(&mut current_text, chunk, i);
            }
        }

        // Add space if needed between chunks (matching chunk_needs_space logic)
        if let Some(next) = sql.chunks.get(i + 1) {
            // Check if we need spacing between these chunks
            let needs_space = chunk_needs_space_for_prepare(chunk, next, &current_text);
            if needs_space {
                current_text.push(' ');
            }
        }
    }

    text_segments.push(CompactString::new(&current_text));

    let text_segments = text_segments.into_boxed_slice();
    let params = params.into_boxed_slice();
    let rendered_sql = render_sql_with_placeholders(&text_segments, &params, |p, idx| {
        // Named placeholders use :name syntax only for SQLite
        // PostgreSQL always uses $N, MySQL always uses ?
        if let Some(name) = p.placeholder.name
            && V::DIALECT == Dialect::SQLite
        {
            Cow::Owned(format!(":{}", name))
        } else {
            V::DIALECT.render_placeholder(idx)
        }
    });

    PreparedStatement {
        text_segments,
        params,
        sql: rendered_sql,
    }
}

/// Check if space is needed between chunks during prepare_render
fn chunk_needs_space_for_prepare<V: SQLParam>(
    current: &SQLChunk<'_, V>,
    next: &SQLChunk<'_, V>,
    current_text: &str,
) -> bool {
    // No space if current text already ends with space
    if current_text.ends_with(' ') {
        return false;
    }

    // No space if next raw text starts with space
    if let SQLChunk::Raw(text) = next
        && text.starts_with(' ')
    {
        return false;
    }

    // Space between word-like chunks
    current.is_word_like() && next.is_word_like()
}
