mod chunk;
mod cte;
mod owned;
mod tokens;

use crate::prelude::*;
use crate::{
    dialect::DialectExt,
    param::{Param, ParamBind},
    placeholder::Placeholder,
    traits::{SQLColumnInfo, SQLParam, SQLTableInfo, ToSQL},
};
pub use chunk::*;
use core::fmt::Display;
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
    const POSITIONAL_PLACEHOLDER: Placeholder = Placeholder::positional();

    // ==================== constructors ====================

    /// Creates an empty SQL fragment
    #[inline]
    pub const fn empty() -> Self {
        Self {
            chunks: SmallVec::new_const(),
        }
    }

    // ==================== constructors ====================

    /// Creates SQL with a single token
    #[inline]
    pub fn token(t: Token) -> Self {
        Self {
            chunks: smallvec::smallvec![SQLChunk::Token(t)],
        }
    }

    /// Creates an empty SQL fragment with pre-allocated chunk capacity.
    #[inline]
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
        V: From<&'a [u8]>,
        V: From<Vec<u8>>,
        V: Into<Cow<'a, V>>,
    {
        match bytes.into() {
            Cow::Borrowed(value) => Self::param(V::from(value)),
            Cow::Owned(value) => Self::param(V::from(value)),
        }
    }

    /// Creates SQL with a named placeholder (no value, for prepared statements)
    #[inline]
    pub fn placeholder(name: &'static str) -> Self {
        Self {
            chunks: smallvec::smallvec![SQLChunk::Param(Param {
                value: None,
                placeholder: Placeholder::colon(name),
            })],
        }
    }

    /// Creates SQL referencing a table
    #[inline]
    pub fn table(table: &'static dyn SQLTableInfo) -> Self {
        Self {
            chunks: smallvec::smallvec![SQLChunk::Table(table)],
        }
    }

    /// Creates SQL referencing a column
    #[inline]
    pub fn column(column: &'static dyn SQLColumnInfo) -> Self {
        Self {
            chunks: smallvec::smallvec![SQLChunk::Column(column)],
        }
    }

    /// Creates SQL for a function call: NAME(args)
    /// Subqueries are automatically wrapped in parentheses: NAME((SELECT ...))
    #[inline]
    pub fn func(name: &'static str, args: SQL<'a, V>) -> Self {
        let args = if args.is_subquery() {
            args.parens()
        } else {
            args
        };
        SQL::raw(name)
            .push(Token::LPAREN)
            .append(args)
            .push(Token::RPAREN)
    }

    // ==================== builder methods ====================

    /// Append another SQL fragment (flat extend)
    #[inline]
    pub fn append(mut self, other: impl Into<SQL<'a, V>>) -> Self {
        #[cfg(feature = "profiling")]
        profile_sql!("append");
        let mut other = other.into();
        if !other.chunks.is_empty() {
            self.chunks.reserve(other.chunks.len());
            self.chunks.extend(other.chunks.drain(..));
        }
        self
    }

    /// Push a single chunk
    #[inline]
    pub fn push(mut self, chunk: impl Into<SQLChunk<'a, V>>) -> Self {
        self.chunks.push(chunk.into());
        self
    }

    /// Pre-allocates capacity for additional chunks
    #[inline]
    pub fn with_capacity(mut self, additional: usize) -> Self {
        self.chunks.reserve(additional);
        self
    }

    // ==================== combinators ====================

    /// Joins multiple SQL fragments with a separator
    pub fn join<T>(sqls: T, separator: Token) -> SQL<'a, V>
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

        let mut result = first.to_sql();
        let (lower, _) = iter.size_hint();
        if lower > 0 {
            // Reserve at least space for separators and minimal chunk growth.
            result.chunks.reserve(lower * 2);
        }
        for item in iter {
            result = result.push(separator).append(item.to_sql());
        }
        result
    }

    /// Wrap in parentheses: (self)
    #[inline]
    pub fn parens(self) -> Self {
        SQL::token(Token::LPAREN).append(self).push(Token::RPAREN)
    }

    /// Check if this SQL fragment is a subquery (starts with SELECT)
    #[inline]
    pub fn is_subquery(&self) -> bool {
        matches!(self.chunks.first(), Some(SQLChunk::Token(Token::SELECT)))
    }

    /// Creates an aliased version: self AS "name"
    pub fn alias(self, name: impl Into<Cow<'a, str>>) -> SQL<'a, V> {
        self.push(Token::AS).push(SQLChunk::Ident(name.into()))
    }

    /// Creates a comma-separated list of parameters
    pub fn param_list<I>(values: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Cow<'a, V>>,
    {
        Self::join(values.into_iter().map(Self::param), Token::COMMA)
    }

    /// Creates a comma-separated list of column assignments: "col" = ?
    pub fn assignments<I, T>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (&'static str, T)>,
        T: Into<Cow<'a, V>>,
    {
        Self::join(
            pairs
                .into_iter()
                .map(|(col, val)| SQL::ident(col).push(Token::EQ).append(SQL::param(val))),
            Token::COMMA,
        )
    }

    // ==================== output methods ====================

    /// Converts to owned version (consuming self to avoid clone)
    pub fn into_owned(self) -> OwnedSQL<V> {
        OwnedSQL::from(self)
    }

    /// Returns the SQL string with dialect-appropriate placeholders
    /// Uses `$1, $2, ...` for PostgreSQL, `?` for SQLite/MySQL
    pub fn sql(&self) -> String {
        #[cfg(feature = "profiling")]
        profile_sql!("sql");
        let capacity = self.estimate_capacity();
        let mut buf = String::with_capacity(capacity);
        self.write_to(&mut buf);
        buf
    }

    /// Write SQL to a buffer with dialect-appropriate placeholders
    /// Uses `$1, $2, ...` for PostgreSQL, `?` or `:name` for SQLite, `?` for MySQL
    /// Named placeholders use `:name` syntax only for SQLite; PostgreSQL always uses `$N`
    pub fn write_to(&self, buf: &mut impl core::fmt::Write) {
        use crate::dialect::Dialect;
        let mut param_index = 1usize;
        for (i, chunk) in self.chunks.iter().enumerate() {
            match chunk {
                SQLChunk::Token(Token::SELECT) => {
                    chunk.write(buf);
                    self.write_select_columns(buf, i);
                }
                SQLChunk::Param(param) => {
                    // Named placeholders use :name syntax only for SQLite
                    // PostgreSQL always uses $N, MySQL always uses ?
                    if let Some(name) = param.placeholder.name
                        && V::DIALECT == Dialect::SQLite
                    {
                        let _ = buf.write_char(':');
                        let _ = buf.write_str(name);
                    } else {
                        let _ = buf.write_str(&V::DIALECT.render_placeholder(param_index));
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
    fn write_select_columns(&self, buf: &mut impl core::fmt::Write, select_index: usize) {
        let chunks = self.chunks.get(select_index + 1..select_index + 3);
        match chunks {
            Some([SQLChunk::Token(Token::FROM), SQLChunk::Table(table)]) => {
                let _ = buf.write_char(' ');
                self.write_qualified_columns(buf, *table);
            }
            Some([SQLChunk::Token(Token::FROM), _]) => {
                let _ = buf.write_char(' ');
                let _ = buf.write_str(Token::STAR.as_str());
            }
            _ => {}
        }
    }

    /// Write fully qualified columns
    pub fn write_qualified_columns(
        &self,
        buf: &mut impl core::fmt::Write,
        table: &dyn SQLTableInfo,
    ) {
        let columns = table.columns();
        if columns.is_empty() {
            let _ = buf.write_char('*');
            return;
        }

        for (i, col) in columns.iter().enumerate() {
            if i > 0 {
                let _ = buf.write_str(", ");
            }
            let _ = buf.write_char('"');
            let _ = buf.write_str(table.name());
            let _ = buf.write_str("\".\"");
            let _ = buf.write_str(col.name());
            let _ = buf.write_char('"');
        }
    }

    fn estimate_capacity(&self) -> usize {
        const PLACEHOLDER_SIZE: usize = 2;
        const IDENT_OVERHEAD: usize = 2;
        const COLUMN_OVERHEAD: usize = 5;
        const ALIAS_OVERHEAD: usize = 6;

        self.chunks
            .iter()
            .map(|chunk| match chunk {
                SQLChunk::Token(t) => t.as_str().len(),
                SQLChunk::Ident(s) => s.len() + IDENT_OVERHEAD,
                SQLChunk::Raw(s) => s.len(),
                SQLChunk::Param { .. } => PLACEHOLDER_SIZE,
                SQLChunk::Table(t) => t.name().len() + IDENT_OVERHEAD,
                SQLChunk::Column(c) => c.table().name().len() + c.name().len() + COLUMN_OVERHEAD,
                SQLChunk::Alias { inner, alias } => {
                    alias.len()
                        + ALIAS_OVERHEAD
                        + match inner.as_ref() {
                            SQLChunk::Ident(s) => s.len() + IDENT_OVERHEAD,
                            SQLChunk::Raw(s) => s.len(),
                            _ => 10,
                        }
                }
            })
            .sum::<usize>()
            + self.chunks.len()
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
    pub fn bind<T: SQLParam + Into<V>>(
        self,
        params: impl IntoIterator<Item: Into<ParamBind<'a, T>>>,
    ) -> SQL<'a, V> {
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

/// Simplified spacing logic
fn chunk_needs_space<V: SQLParam>(current: &SQLChunk<'_, V>, next: &SQLChunk<'_, V>) -> bool {
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
        (_, SQLChunk::Token(Token::RPAREN | Token::COMMA | Token::SEMI | Token::DOT)) => false,
        // No space after opening punctuation
        (SQLChunk::Token(Token::LPAREN | Token::DOT), _) => false,
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

impl<'a, V: SQLParam> Default for SQL<'a, V> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<'a, V: SQLParam + 'a> From<&'a str> for SQL<'a, V> {
    fn from(s: &'a str) -> Self {
        SQL::raw(s)
    }
}

impl<'a, V: SQLParam> From<Token> for SQL<'a, V> {
    fn from(value: Token) -> Self {
        SQL::token(value)
    }
}

impl<'a, V: SQLParam + 'a> AsRef<SQL<'a, V>> for SQL<'a, V> {
    fn as_ref(&self) -> &SQL<'a, V> {
        self
    }
}

impl<'a, V: SQLParam + core::fmt::Display> Display for SQL<'a, V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Collect params for Debug formatting (iterator can't be used with :?)
        let params: Vec<_> = self.params().collect();
        write!(f, r#"sql: "{}", params: {:?}"#, self.sql(), params)
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for SQL<'a, V> {
    fn to_sql(&self) -> SQL<'a, V> {
        self.clone()
    }
}

impl<'a, V: SQLParam, T> FromIterator<T> for SQL<'a, V>
where
    SQLChunk<'a, V>: From<T>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let chunks = SmallVec::from_iter(iter.into_iter().map(SQLChunk::from));
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
