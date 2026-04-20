mod chunk;
mod comment;
mod cte;
mod owned;
mod tokens;

use crate::prelude::*;
use crate::{
    param::{Param, ParamBind},
    placeholder::Placeholder,
    traits::{SQLParam, ToSQL},
};
pub use chunk::*;
pub use comment::{comment, comment_tags};
use core::fmt::{Display, Write};
pub use owned::*;
use smallvec::SmallVec;
pub use tokens::*;

#[cfg(feature = "profiling")]
use crate::profile_sql;

/// SQL fragment builder with flat chunk storage.
///
/// Uses `SmallVec<[SQLChunk; 8]>` for inline storage of typical SQL fragments
/// without heap allocation.
#[derive(Debug, Clone)]
pub struct SQL<'a, V: SQLParam> {
    pub chunks: SmallVec<[SQLChunk<'a, V>; 8]>,
}

impl<'a, V: SQLParam> SQL<'a, V> {
    const POSITIONAL_PLACEHOLDER: Placeholder = Placeholder::anonymous();

    // ==================== constructors ====================

    /// Creates an empty SQL fragment
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            chunks: SmallVec::new_const(),
        }
    }

    // ==================== constructors ====================

    /// Creates SQL with a single token
    #[inline]
    #[must_use]
    pub fn token(t: Token) -> Self {
        Self {
            chunks: smallvec::smallvec![SQLChunk::Token(t)],
        }
    }

    /// Creates an empty SQL fragment with pre-allocated chunk capacity.
    #[inline]
    #[must_use]
    pub fn with_capacity_chunks(capacity: usize) -> Self {
        Self {
            chunks: SmallVec::with_capacity(capacity),
        }
    }

    /// Creates SQL with a quoted identifier
    #[inline]
    pub fn ident(name: impl Into<Cow<'a, str>>) -> Self {
        Self {
            chunks: smallvec::smallvec![SQLChunk::Ident(name.into())],
        }
    }

    /// Creates SQL with raw text (unquoted)
    #[inline]
    pub fn raw(text: impl Into<Cow<'a, str>>) -> Self {
        Self {
            chunks: smallvec::smallvec![SQLChunk::Raw(text.into())],
        }
    }

    /// Creates SQL with a single unsigned integer literal.
    #[inline]
    #[must_use]
    pub fn number(value: usize) -> Self {
        Self {
            chunks: smallvec::smallvec![SQLChunk::Number(value)],
        }
    }

    /// Creates SQL with a single parameter value
    #[inline]
    pub fn param(value: impl Into<Cow<'a, V>>) -> Self {
        Self {
            chunks: smallvec::smallvec![SQLChunk::Param(Param {
                value: Some(value.into()),
                placeholder: Self::POSITIONAL_PLACEHOLDER,
            })],
        }
    }

    /// Creates SQL with a binary parameter value (BLOB/bytea)
    ///
    /// Prefer this over `SQL::param(Vec<u8>)` to avoid list semantics.
    #[inline]
    pub fn bytes(bytes: impl Into<Cow<'a, [u8]>>) -> Self
    where
        V: From<&'a [u8]> + From<Vec<u8>> + Into<Cow<'a, V>>,
    {
        match bytes.into() {
            Cow::Borrowed(value) => Self::param(V::from(value)),
            Cow::Owned(value) => Self::param(V::from(value)),
        }
    }

    /// Creates SQL referencing a table
    #[inline]
    #[must_use]
    pub fn table(table: TableRef) -> Self {
        Self {
            chunks: smallvec::smallvec![SQLChunk::Table(table)],
        }
    }

    /// Creates SQL referencing a column
    #[inline]
    #[must_use]
    pub fn column(column: ColumnRef) -> Self {
        Self {
            chunks: smallvec::smallvec![SQLChunk::Column(column)],
        }
    }

    /// Creates SQL for a function call: NAME(args)
    /// Subqueries are automatically wrapped in parentheses: NAME((SELECT ...))
    #[inline]
    pub fn func(name: &'static str, args: Self) -> Self {
        let args = args.parens_if_subquery();
        SQL::raw(name)
            .push(Token::LPAREN)
            .append(args)
            .push(Token::RPAREN)
    }

    // ==================== builder methods ====================

    /// Append another SQL fragment (flat extend)
    #[inline]
    #[must_use]
    pub fn append(mut self, other: impl Into<Self>) -> Self {
        #[cfg(feature = "profiling")]
        profile_sql!("append");
        let other = other.into();

        if self.chunks.is_empty() {
            return other;
        }
        if other.chunks.is_empty() {
            return self;
        }

        self.chunks.extend(other.chunks);
        self
    }

    #[inline]
    pub fn append_mut(&mut self, other: impl Into<Self>) {
        #[cfg(feature = "profiling")]
        profile_sql!("append_mut");
        let other = other.into();

        if self.chunks.is_empty() {
            self.chunks = other.chunks;
            return;
        }
        if other.chunks.is_empty() {
            return;
        }

        self.chunks.extend(other.chunks);
    }

    /// Push a single chunk
    #[inline]
    #[must_use]
    pub fn push(mut self, chunk: impl Into<SQLChunk<'a, V>>) -> Self {
        self.chunks.push(chunk.into());
        self
    }

    #[inline]
    pub fn push_mut(&mut self, chunk: impl Into<SQLChunk<'a, V>>) {
        self.chunks.push(chunk.into());
    }

    /// Pre-allocates capacity for additional chunks
    #[inline]
    #[must_use]
    pub fn with_capacity(mut self, additional: usize) -> Self {
        self.chunks.reserve(additional);
        self
    }

    // ==================== combinators ====================

    /// Joins multiple SQL fragments with a separator
    pub fn join<T>(sqls: T, separator: Token) -> Self
    where
        T: IntoIterator,
        T::Item: ToSQL<'a, V>,
    {
        #[cfg(feature = "profiling")]
        profile_sql!("join");

        let mut iter = sqls.into_iter();
        let Some(first) = iter.next() else {
            return SQL::empty();
        };

        let mut result = first.into_sql();
        let (lower, upper) = iter.size_hint();
        if let Some(upper) = upper {
            result.chunks.reserve(upper.saturating_mul(2));
        } else if lower > 0 {
            result.chunks.reserve(lower * 2);
        }

        for item in iter {
            result.chunks.push(SQLChunk::Token(separator));
            let other = item.into_sql();
            if !other.chunks.is_empty() {
                result.chunks.extend(other.chunks);
            }
        }
        result
    }

    /// Wrap in parentheses: (self)
    #[inline]
    #[must_use]
    pub fn parens(self) -> Self {
        SQL::token(Token::LPAREN).append(self).push(Token::RPAREN)
    }

    /// Wrap this SQL fragment in parentheses only when it is a subquery.
    #[inline]
    #[must_use]
    pub fn parens_if_subquery(self) -> Self {
        if self.is_subquery() {
            self.parens()
        } else {
            self
        }
    }

    /// Check if this SQL fragment is a subquery (starts with SELECT/WITH).
    #[inline]
    pub fn is_subquery(&self) -> bool {
        matches!(
            self.chunks.first(),
            Some(SQLChunk::Token(Token::SELECT | Token::WITH))
        )
    }

    /// Creates an aliased version: self AS "name"
    #[must_use]
    pub fn alias(self, name: impl Into<Cow<'a, str>>) -> Self {
        self.push(Token::AS).push(SQLChunk::Ident(name.into()))
    }

    /// Creates a comma-separated list of parameters.
    /// Builds chunks directly without intermediate SQL allocations.
    pub fn param_list<I>(values: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Cow<'a, V>>,
    {
        let iter = values.into_iter();
        let (lower, _) = iter.size_hint();
        let mut chunks = SmallVec::with_capacity(lower.saturating_mul(2));
        for (i, v) in iter.enumerate() {
            if i > 0 {
                chunks.push(SQLChunk::Token(Token::COMMA));
            }
            chunks.push(SQLChunk::Param(Param {
                value: Some(v.into()),
                placeholder: Self::POSITIONAL_PLACEHOLDER,
            }));
        }
        SQL { chunks }
    }

    /// Creates a comma-separated list of column assignments: "col" = ?
    /// Builds chunks directly without intermediate SQL allocations.
    pub fn assignments<I, T>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (&'static str, T)>,
        T: Into<Cow<'a, V>>,
    {
        let iter = pairs.into_iter();
        let (lower, _) = iter.size_hint();
        // Each assignment: Ident + EQ + Param = 3 chunks, plus commas
        let mut chunks = SmallVec::with_capacity(lower.saturating_mul(4));
        for (i, (col, val)) in iter.enumerate() {
            if i > 0 {
                chunks.push(SQLChunk::Token(Token::COMMA));
            }
            chunks.push(SQLChunk::Ident(Cow::Borrowed(col)));
            chunks.push(SQLChunk::Token(Token::EQ));
            chunks.push(SQLChunk::Param(Param {
                value: Some(val.into()),
                placeholder: Self::POSITIONAL_PLACEHOLDER,
            }));
        }
        SQL { chunks }
    }

    /// Creates a comma-separated list of column assignments from pre-built SQL fragments: "col" = <sql>
    ///
    /// Unlike `assignments()` which wraps each value in `SQL::param()`, this variant
    /// accepts pre-built `SQL` fragments, preserving placeholders and raw expressions.
    /// Builds chunks directly without intermediate SQL allocations.
    pub fn assignments_sql<I>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (&'static str, Self)>,
    {
        let iter = pairs.into_iter();
        let (lower, _) = iter.size_hint();
        let mut chunks = SmallVec::with_capacity(lower.saturating_mul(4));
        for (i, (col, sql)) in iter.enumerate() {
            if i > 0 {
                chunks.push(SQLChunk::Token(Token::COMMA));
            }
            chunks.push(SQLChunk::Ident(Cow::Borrowed(col)));
            chunks.push(SQLChunk::Token(Token::EQ));
            chunks.extend(sql.chunks);
        }
        SQL { chunks }
    }

    // ==================== output methods ====================

    /// Maps parameter values from type `V` to type `U` using the provided function.
    ///
    /// Only `Param` chunks are affected; all other chunks pass through unchanged.
    /// This is useful for converting between owned and borrowed value types
    /// (e.g. `OwnedPostgresValue` → `PostgresValue<'a>`).
    pub fn map_params<U: SQLParam>(self, mut f: impl FnMut(V) -> U) -> SQL<'a, U> {
        let chunks = self
            .chunks
            .into_iter()
            .map(|chunk| match chunk {
                SQLChunk::Token(t) => SQLChunk::Token(t),
                SQLChunk::Ident(s) => SQLChunk::Ident(s),
                SQLChunk::Raw(s) => SQLChunk::Raw(s),
                SQLChunk::Number(n) => SQLChunk::Number(n),
                SQLChunk::Param(param) => SQLChunk::Param(Param::new(
                    param.placeholder,
                    param.value.map(|cow| Cow::Owned(f(cow.into_owned()))),
                )),
                SQLChunk::Table(t) => SQLChunk::Table(t),
                SQLChunk::Column(c) => SQLChunk::Column(c),
            })
            .collect();
        SQL { chunks }
    }

    /// Converts to owned version (consuming self to avoid clone)
    pub fn into_owned(self) -> OwnedSQL<V> {
        OwnedSQL::from(self)
    }

    /// Returns the SQL string with dialect-appropriate placeholders.
    /// Uses `$1, $2, ...` for `PostgreSQL`, `:name` or `?` for `SQLite`, `?` for `MySQL`.
    pub fn sql(&self) -> String {
        #[cfg(feature = "profiling")]
        profile_sql!("sql");
        #[cfg(feature = "profiling")]
        crate::drizzle_profile_scope!("sql_render", "sql.estimate");
        let sql_cap = self.chunks.len().saturating_mul(8).max(128);
        let mut buf = String::with_capacity(sql_cap);
        self.write_to(&mut buf);
        buf
    }

    /// Generates the SQL string and collects parameter references in a single pass.
    ///
    /// This is the preferred method for driver execution paths since it avoids
    /// iterating the chunk list twice (once for `sql()`, once for `params()`).
    pub fn build(&self) -> (String, SmallVec<[&V; 8]>) {
        self.build_with(crate::dialect::ParamStyle::for_dialect(V::DIALECT))
    }

    /// Same as [`build`](Self::build) but lets the caller override the
    /// placeholder style. Drivers that speak the dialect but bind parameters
    /// differently (e.g. AWS Data API on Postgres) use this to emit
    /// `:1, :2, ...` instead of `$1, $2, ...` without any post-hoc rewriting.
    pub fn build_with(&self, style: crate::dialect::ParamStyle) -> (String, SmallVec<[&V; 8]>) {
        use crate::dialect::Dialect;

        #[cfg(feature = "profiling")]
        crate::drizzle_profile_scope!("sql_render", "build");
        #[cfg(feature = "profiling")]
        crate::drizzle_profile_scope!("sql_render", "build.estimate");
        let sql_cap = self.chunks.len().saturating_mul(8).max(128);
        let param_cap = self.chunks.len().saturating_div(8).max(8);
        let mut buf = String::with_capacity(sql_cap);
        let mut params: SmallVec<[&V; 8]> = SmallVec::with_capacity(param_cap);
        let mut param_index = 1usize;

        #[cfg(feature = "profiling")]
        crate::drizzle_profile_scope!("sql_render", "build.render");
        for (i, chunk) in self.chunks.iter().enumerate() {
            match chunk {
                SQLChunk::Token(Token::SELECT) => {
                    chunk.write(&mut buf);
                    self.write_select_columns(&mut buf, i);
                }
                SQLChunk::Param(param) => {
                    if let Some(name) = param.placeholder.name
                        && V::DIALECT == Dialect::SQLite
                    {
                        let _ = buf.write_char(':');
                        let _ = buf.write_str(name);
                    } else {
                        style.write(param_index, &mut buf);
                    }
                    param_index += 1;
                    if let Some(value) = &param.value {
                        params.push(value.as_ref());
                    }
                }
                _ => chunk.write(&mut buf),
            }

            if self.needs_space(i) {
                let _ = buf.write_char(' ');
            }
        }

        (buf, params)
    }

    /// Write SQL to a buffer with dialect-appropriate placeholders.
    /// Uses `$1, $2, ...` for `PostgreSQL`, `?` or `:name` for `SQLite`, `?` for `MySQL`.
    pub fn write_to(&self, buf: &mut impl core::fmt::Write) {
        self.write_to_with(buf, crate::dialect::ParamStyle::for_dialect(V::DIALECT));
    }

    /// Same as [`write_to`](Self::write_to) but with a caller-chosen
    /// placeholder style.
    pub fn write_to_with(
        &self,
        buf: &mut impl core::fmt::Write,
        style: crate::dialect::ParamStyle,
    ) {
        use crate::dialect::Dialect;

        #[cfg(feature = "profiling")]
        crate::drizzle_profile_scope!("sql_render", "write_to");
        let mut param_index = 1usize;
        for (i, chunk) in self.chunks.iter().enumerate() {
            match chunk {
                SQLChunk::Token(Token::SELECT) => {
                    chunk.write(buf);
                    self.write_select_columns(buf, i);
                }
                SQLChunk::Param(param) => {
                    if let Some(name) = param.placeholder.name
                        && V::DIALECT == Dialect::SQLite
                    {
                        let _ = buf.write_char(':');
                        let _ = buf.write_str(name);
                    } else {
                        style.write(param_index, buf);
                    }
                    param_index += 1;
                }
                _ => chunk.write(buf),
            }

            if self.needs_space(i) {
                let _ = buf.write_char(' ');
            }
        }
    }

    /// Write a single chunk with pattern detection
    pub fn write_chunk_to(
        &self,
        buf: &mut impl core::fmt::Write,
        chunk: &SQLChunk<'a, V>,
        index: usize,
    ) {
        match chunk {
            SQLChunk::Token(Token::SELECT) => {
                chunk.write(buf);
                self.write_select_columns(buf, index);
            }
            _ => chunk.write(buf),
        }
    }

    /// Write appropriate columns for SELECT statement
    pub(crate) fn write_select_columns(
        &self,
        buf: &mut impl core::fmt::Write,
        select_index: usize,
    ) {
        let chunks = self.chunks.get(select_index + 1..select_index + 3);
        match chunks {
            Some([SQLChunk::Token(Token::FROM), SQLChunk::Table(table)]) => {
                let _ = buf.write_char(' ');
                Self::write_qualified_columns(buf, table);
            }
            Some([SQLChunk::Token(Token::FROM), _]) => {
                let _ = buf.write_char(' ');
                let _ = buf.write_str(Token::STAR.as_str());
            }
            _ => {}
        }
    }

    /// Write fully qualified columns for a table
    pub fn write_qualified_columns(buf: &mut impl core::fmt::Write, table: &TableRef) {
        if table.column_names.is_empty() {
            let _ = buf.write_char('*');
            return;
        }

        for (i, col_name) in table.column_names.iter().enumerate() {
            if i > 0 {
                let _ = buf.write_str(", ");
            }
            chunk::write_quoted_ident(buf, table.name);
            let _ = buf.write_char('.');
            chunk::write_quoted_ident(buf, col_name);
        }
    }

    /// Simplified spacing logic
    fn needs_space(&self, index: usize) -> bool {
        let Some(next) = self.chunks.get(index + 1) else {
            return false;
        };

        let current = &self.chunks[index];
        chunk_needs_space(current, next)
    }

    /// Returns an iterator over references to parameter values
    /// (avoids allocating a Vec - callers can collect if needed)
    pub fn params(&self) -> impl Iterator<Item = &V> {
        self.chunks.iter().filter_map(|chunk| {
            if let SQLChunk::Param(Param {
                value: Some(value), ..
            }) = chunk
            {
                Some(value.as_ref())
            } else {
                None
            }
        })
    }

    /// Bind named parameters
    #[must_use]
    pub fn bind<T: SQLParam + Into<V>>(
        self,
        params: impl IntoIterator<Item: Into<ParamBind<'a, T>>>,
    ) -> Self {
        #[cfg(feature = "profiling")]
        profile_sql!("bind");

        let param_map: HashMap<&str, V> = params
            .into_iter()
            .map(Into::into)
            .map(|p| (p.name, p.value.into()))
            .collect();

        let bound_chunks: SmallVec<[SQLChunk<'a, V>; 8]> = self
            .chunks
            .into_iter()
            .map(|chunk| match chunk {
                SQLChunk::Param(mut param) => {
                    if let Some(name) = param.placeholder.name
                        && let Some(value) = param_map.get(name)
                    {
                        param.value = Some(Cow::Owned(value.clone()));
                    }
                    SQLChunk::Param(param)
                }
                other => other,
            })
            .collect();

        SQL {
            chunks: bound_chunks,
        }
    }
}

/// Canonical spacing logic for SQL chunk rendering.
/// Used by both `SQL::write_to()` and `prepare_render()`.
pub(crate) fn chunk_needs_space<V: SQLParam>(
    current: &SQLChunk<'_, V>,
    next: &SQLChunk<'_, V>,
) -> bool {
    // No space if current raw text ends with space
    if let SQLChunk::Raw(text) = current
        && text.ends_with(' ')
    {
        return false;
    }

    // No space if next raw text starts with space
    if let SQLChunk::Raw(text) = next
        && text.starts_with(' ')
    {
        return false;
    }

    match (current, next) {
        // No space before closing/separator punctuation
        // or after opening punctuation
        (_, SQLChunk::Token(Token::RPAREN | Token::COMMA | Token::SEMI | Token::DOT))
        | (SQLChunk::Token(Token::LPAREN | Token::DOT), _) => false,
        // Space after comma
        (SQLChunk::Token(Token::COMMA), _) => true,
        // Space after closing paren if next is word-like (e.g., ") FROM")
        (SQLChunk::Token(Token::RPAREN), next) => next.is_word_like(),
        // Space before opening paren if preceded by word-like (e.g., "AS (")
        (current, SQLChunk::Token(Token::LPAREN)) => current.is_word_like(),
        // Space around comparison/arithmetic operators
        (SQLChunk::Token(t), _) if t.is_operator() => true,
        (_, SQLChunk::Token(t)) if t.is_operator() => true,
        // Space between all word-like chunks
        _ => current.is_word_like() && next.is_word_like(),
    }
}

// ==================== trait implementations ====================

impl<V: SQLParam> Default for SQL<'_, V> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<'a, V: SQLParam + 'a> From<&'a str> for SQL<'a, V> {
    fn from(s: &'a str) -> Self {
        SQL::raw(s)
    }
}

impl<V: SQLParam> From<Token> for SQL<'_, V> {
    fn from(value: Token) -> Self {
        SQL::token(value)
    }
}

impl<'a, V: SQLParam + 'a> AsRef<Self> for SQL<'a, V> {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<V: SQLParam + core::fmt::Display> Display for SQL<'_, V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Collect params for Debug formatting (iterator can't be used with :?)
        let params: Vec<_> = self.params().collect();
        write!(f, r#"sql: "{}", params: {:?}"#, self.sql(), params)
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for SQL<'a, V> {
    fn to_sql(&self) -> Self {
        self.clone()
    }

    fn into_sql(self) -> Self {
        self
    }
}

impl<'a, V: SQLParam, T> FromIterator<T> for SQL<'a, V>
where
    SQLChunk<'a, V>: From<T>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let chunks = iter
            .into_iter()
            .map(SQLChunk::from)
            .collect::<SmallVec<_>>();
        Self { chunks }
    }
}

impl<'a, V: SQLParam> IntoIterator for SQL<'a, V> {
    type Item = SQLChunk<'a, V>;
    type IntoIter = smallvec::IntoIter<[SQLChunk<'a, V>; 8]>;

    fn into_iter(self) -> Self::IntoIter {
        self.chunks.into_iter()
    }
}
