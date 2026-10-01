mod owned;
pub use owned::OwnedPreparedStatement;

use crate::prelude::*;
use crate::{
    error::DrizzleError,
    param::{Param, ParamBind},
    sql::{SQL, SQLChunk, SQLiteNamedParams},
    traits::{SQLParam, ToSQL},
};
use compact_str::CompactString;
use core::fmt;
use smallvec::SmallVec;

/// A statement rendered once, whose placeholders are bound each time it
/// runs.
///
/// Create one with [`prepare_render`]; drivers wrap it in their own prepared
/// statement types. The SQL is stored as text segments with one parameter
/// between each pair: `[text, param, text, param, text]`. Parameters that
/// already had a value when the statement was rendered keep it.
///
/// # Examples
///
/// ```
/// use drizzle_core::prepared::prepare_render;
/// use drizzle_core::{ParamBind, Placeholder, SQL, ToSQL};
/// # use drizzle_core::{Dialect, SQLParam, SQLiteDialect};
/// # use std::borrow::Cow;
/// # #[derive(Debug, Clone, PartialEq)]
/// # struct Value(i64);
/// # impl SQLParam for Value {
/// #     const DIALECT: Dialect = Dialect::SQLite;
/// #     type DialectMarker = SQLiteDialect;
/// # }
/// # impl From<Value> for Cow<'_, Value> {
/// #     fn from(value: Value) -> Self { Cow::Owned(value) }
/// # }
///
/// let sql: SQL<'_, Value> = SQL::raw("SELECT * FROM users WHERE id =")
///     .append(Placeholder::named("id").to_sql());
/// let prepared = prepare_render(&sql);
/// assert_eq!(prepared.sql(), "SELECT * FROM users WHERE id = :id");
/// assert_eq!(prepared.external_param_count(), 1);
///
/// let (text, values) = prepared.bind([ParamBind::new("id", Value(5))])?;
/// assert_eq!(text, "SELECT * FROM users WHERE id = :id");
/// assert_eq!(values.collect::<Vec<_>>(), [Value(5)]);
/// # Ok::<(), drizzle_core::error::DrizzleError>(())
/// ```
#[derive(Debug, Clone)]
pub struct PreparedStatement<'a, V: SQLParam> {
    /// Rendered SQL text between the parameters; one more than `params`.
    pub text_segments: Box<[CompactString]>,
    /// The parameters, in order.
    pub params: Box<[Param<'a, V>]>,
    /// The full SQL text, with the dialect's placeholders.
    pub sql: CompactString,
}

impl<V: SQLParam> From<OwnedPreparedStatement<V>> for PreparedStatement<'_, V> {
    fn from(value: OwnedPreparedStatement<V>) -> Self {
        Self {
            text_segments: value.text_segments,
            params: value.params.iter().map(|v| v.clone().into()).collect(),
            sql: value.sql,
        }
    }
}

impl<V: SQLParam> core::fmt::Display for PreparedStatement<'_, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.sql())
    }
}

/// Matches `param_binds` to `params` and returns the values to send, in
/// order. See [`PreparedStatement::bind`] for the errors.
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
    let mut sqlite_names = SQLiteNamedParams::default();

    for param in params {
        // SQLite renders a named parameter as `:name` and gives each distinct
        // name one slot, so only its first occurrence binds a value.
        if V::DIALECT == crate::dialect::Dialect::SQLite
            && let Some(name) = param_name_fn(param)
            && sqlite_names.is_repeat(name)
        {
            continue;
        }

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
    /// Returns how many bindings [`bind`](Self::bind) expects.
    ///
    /// Counts parameters without a value, with each placeholder name counted
    /// once, since one binding fills every use of a name.
    #[must_use]
    pub fn external_param_count(&self) -> usize {
        let mut named = HashSet::<&str>::new();
        let mut positional = 0usize;
        for param in &self.params {
            if param.value.is_some() {
                continue;
            }
            match param.placeholder.name {
                Some(name) if !name.is_empty() => {
                    named.insert(name);
                }
                _ => positional += 1,
            }
        }
        named.len() + positional
    }

    /// Binds values to the placeholders and returns the SQL text with the
    /// values to send, in order.
    ///
    /// Named bindings match placeholders by name; unnamed ones
    /// ([`ParamBind::positional`]) fill unnamed placeholders in order. For
    /// SQLite, a name used more than once is sent once.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ParameterError`] when a name is bound twice,
    /// a placeholder has no binding, or a binding matches no placeholder.
    pub fn bind<T: SQLParam + Into<V>>(
        &self,
        param_binds: impl IntoIterator<Item = ParamBind<'a, T>>,
    ) -> crate::error::Result<(&str, impl Iterator<Item = V>)> {
        let bound_params = bind_values_internal(
            &self.params,
            param_binds,
            |p| p.placeholder.name,
            |p| p.value.as_ref().map(core::convert::AsRef::as_ref),
        )?;

        Ok((self.sql.as_str(), bound_params.into_iter()))
    }

    /// Returns the SQL text, with the dialect's placeholders.
    #[must_use]
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
/// Renders `sql` into a [`PreparedStatement`], splitting the text around its
/// parameters.
pub fn prepare_render<'a, V: SQLParam>(sql: &SQL<'a, V>) -> PreparedStatement<'a, V> {
    use crate::dialect::{Dialect, write_placeholder};
    use crate::sql::chunk_needs_space;

    #[cfg(feature = "profiling")]
    crate::drizzle_profile_scope!("prepared", "prepare_render");

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
        let current_text_ends_with_space = if let SQLChunk::Param(param) = chunk {
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
        } else {
            sql.write_chunk_to(&mut current_text, chunk, i);
            matches!(chunk, SQLChunk::Raw(text) if text.ends_with(' '))
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
