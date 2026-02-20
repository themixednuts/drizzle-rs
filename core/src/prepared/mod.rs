mod owned;
pub use owned::OwnedPreparedStatement;

use crate::prelude::*;
use crate::{
    error::DrizzleError,
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
) -> crate::error::Result<SmallVec<[V; 8]>>
where
    V: SQLParam + Clone,
    T: SQLParam + Into<V>,
{
    #[cfg(feature = "profiling")]
    crate::drizzle_profile_scope!("prepared", "bind_values_internal");
    let param_binds = param_binds.into_iter();
    let (binds_lower, binds_upper) = param_binds.size_hint();

    let mut expected_named = HashMap::<&str, usize>::new();
    let mut expected_positional = 0usize;
    for param in params {
        if param_value_fn(param).is_some() {
            continue;
        }

        match param_name_fn(param) {
            Some(name) if !name.is_empty() => {
                *expected_named.entry(name).or_insert(0) += 1;
            }
            _ => expected_positional += 1,
        }
    }

    let mut param_map = HashMap::<&str, V>::with_capacity(expected_named.len().max(binds_lower));

    let mut positional_params: SmallVec<[V; 8]> =
        SmallVec::with_capacity(binds_upper.unwrap_or(binds_lower));

    for bind in param_binds {
        if bind.name.is_empty() {
            positional_params.push(bind.value.into());
        } else if param_map.insert(bind.name, bind.value.into()).is_some() {
            return Err(DrizzleError::ParameterError(
                format!("Duplicate parameter binding: '{}'", bind.name).into(),
            ));
        }
    }

    if positional_params.len() < expected_positional {
        return Err(DrizzleError::ParameterError(
            format!(
                "Missing positional parameter(s): expected {}, got {}",
                expected_positional,
                positional_params.len()
            )
            .into(),
        ));
    }
    if positional_params.len() > expected_positional {
        return Err(DrizzleError::ParameterError(
            format!(
                "Unexpected positional parameter(s): expected {}, got {}",
                expected_positional,
                positional_params.len()
            )
            .into(),
        ));
    }

    let mut missing_named: SmallVec<[&str; 8]> = expected_named
        .keys()
        .filter(|name| !param_map.contains_key(**name))
        .copied()
        .collect();
    if !missing_named.is_empty() {
        missing_named.sort_unstable();
        return Err(DrizzleError::ParameterError(
            format!("Missing named parameter(s): {}", missing_named.join(", ")).into(),
        ));
    }

    let mut extra_named: SmallVec<[&str; 8]> = param_map
        .keys()
        .filter(|name| !expected_named.contains_key(**name))
        .copied()
        .collect();
    if !extra_named.is_empty() {
        extra_named.sort_unstable();
        return Err(DrizzleError::ParameterError(
            format!("Unexpected named parameter(s): {}", extra_named.join(", ")).into(),
        ));
    }

    let mut positional_iter = positional_params.into_iter();

    let mut bound_params = SmallVec::<[V; 8]>::with_capacity(params.len());

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

    Ok(bound_params)
}

impl<'a, V: SQLParam> PreparedStatement<'a, V> {
    /// Returns the number of external parameter bindings expected.
    /// This counts params that need external binding (no pre-set value),
    /// deduplicating named params since one binding satisfies all uses.
    pub fn external_param_count(&self) -> usize {
        let mut named = HashMap::<&str, ()>::new();
        let mut positional = 0usize;
        for param in self.params.iter() {
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

    /// Bind parameters and return SQL with dialect-appropriate placeholders.
    /// Uses `$1, $2, ...` for PostgreSQL, `:name` or `?` for SQLite, `?` for MySQL.
    pub fn bind<T: SQLParam + Into<V>>(
        &self,
        param_binds: impl IntoIterator<Item = ParamBind<'a, T>>,
    ) -> crate::error::Result<(&str, impl Iterator<Item = V>)> {
        let bound_params = bind_values_internal(
            &self.params,
            param_binds,
            |p| p.placeholder.name,
            |p| p.value.as_ref().map(|v| v.as_ref()),
        )?;

        Ok((self.sql.as_str(), bound_params.into_iter()))
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
    #[cfg(feature = "profiling")]
    crate::drizzle_profile_scope!("prepared", "prepare_render");
    use crate::dialect::{Dialect, write_placeholder};
    use crate::sql::chunk_needs_space;

    if !sql
        .chunks
        .iter()
        .any(|chunk| matches!(chunk, SQLChunk::Param(_)))
    {
        #[cfg(feature = "profiling")]
        crate::drizzle_profile_scope!("prepared", "prepare_render.no_params");
        let rendered_sql = CompactString::new(sql.sql());
        return PreparedStatement {
            text_segments: vec![rendered_sql.clone()].into_boxed_slice(),
            params: Vec::new().into_boxed_slice(),
            sql: rendered_sql,
        };
    }

    #[cfg(feature = "profiling")]
    crate::drizzle_profile_scope!("prepared", "prepare_render.scan");
    let mut text_segments = Vec::new();
    let mut params = Vec::new();
    let mut current_text = String::new();
    let mut rendered_sql = String::with_capacity(sql.chunks.len().saturating_mul(8).max(64));
    let mut param_index = 1usize;

    for (i, chunk) in sql.chunks.iter().enumerate() {
        let current_text_ends_with_space = match chunk {
            SQLChunk::Param(param) => {
                text_segments.push(CompactString::new(&current_text));
                rendered_sql.push_str(&current_text);
                current_text.clear();
                params.push(param.clone());

                if let Some(name) = param.placeholder.name
                    && V::DIALECT == Dialect::SQLite
                {
                    rendered_sql.push(':');
                    rendered_sql.push_str(name);
                } else {
                    write_placeholder(V::DIALECT, param_index, &mut rendered_sql);
                }
                param_index += 1;
                false
            }
            _ => {
                sql.write_chunk_to(&mut current_text, chunk, i);
                matches!(chunk, SQLChunk::Raw(text) if text.ends_with(' '))
            }
        };

        // Use the canonical spacing logic, with an extra check for trailing spaces
        // already in the accumulated text buffer
        if let Some(next) = sql.chunks.get(i + 1)
            && !current_text_ends_with_space
            && chunk_needs_space(chunk, next)
        {
            current_text.push(' ');
        }
    }

    text_segments.push(CompactString::new(&current_text));
    rendered_sql.push_str(&current_text);

    #[cfg(feature = "profiling")]
    crate::drizzle_profile_scope!("prepared", "prepare_render.finalize");
    let text_segments = text_segments.into_boxed_slice();
    let params = params.into_boxed_slice();
    let rendered_sql = CompactString::new(rendered_sql);

    PreparedStatement {
        text_segments,
        params,
        sql: rendered_sql,
    }
}
