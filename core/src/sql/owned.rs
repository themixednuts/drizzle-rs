use crate::prelude::*;
use crate::sql::{ColumnRef, ColumnSqlRef, TableRef, TableSqlRef};
use crate::{OwnedParam, SQL, SQLChunk, SQLParam, ToSQL, Token};
use smallvec::SmallVec;

/// A [`SQLChunk`] that owns all its data, so it has no lifetime.
#[derive(Debug, Clone)]
pub enum OwnedSQLChunk<V: SQLParam> {
    /// A keyword or punctuation token.
    Token(Token),
    /// A quoted identifier.
    Ident(Box<str>),
    /// Raw SQL text, written as-is.
    Raw(Box<str>),
    /// An unsigned integer literal.
    Number(usize),
    /// A placeholder with an optional bound value.
    Param(OwnedParam<V>),
    /// A table reference.
    Table(TableSqlRef),
    /// A column reference.
    Column(ColumnSqlRef),
}

impl<V: SQLParam> OwnedSQLChunk<V> {
    /// Creates a token chunk.
    #[inline]
    #[must_use]
    pub const fn token(t: Token) -> Self {
        Self::Token(t)
    }

    /// Creates a table chunk.
    #[inline]
    #[must_use]
    pub const fn table(table: TableRef) -> Self {
        Self::Table(TableSqlRef::from_table_ref(table))
    }

    /// Creates a column chunk.
    #[inline]
    #[must_use]
    pub const fn column(column: ColumnRef) -> Self {
        Self::Column(ColumnSqlRef::from_column_ref(column))
    }

    /// Creates a quoted identifier from a runtime string.
    #[inline]
    pub fn ident(name: impl Into<Box<str>>) -> Self {
        Self::Ident(name.into())
    }

    /// Creates raw SQL text from a runtime string.
    #[inline]
    pub fn raw(text: impl Into<Box<str>>) -> Self {
        Self::Raw(text.into())
    }

    /// Creates a parameter chunk.
    #[inline]
    pub const fn param(value: OwnedParam<V>) -> Self {
        Self::Param(value)
    }
}

impl<'a, V: SQLParam> From<SQLChunk<'a, V>> for OwnedSQLChunk<V> {
    fn from(value: SQLChunk<'a, V>) -> Self {
        match value {
            SQLChunk::Token(token) => Self::Token(token),
            SQLChunk::Ident(cow) => Self::Ident(cow.into_owned().into_boxed_str()),
            SQLChunk::Raw(cow) => Self::Raw(cow.into_owned().into_boxed_str()),
            SQLChunk::Number(value) => Self::Number(value),
            SQLChunk::Param(param) => Self::Param(param.into()),
            SQLChunk::Table(t) => Self::Table(t),
            SQLChunk::Column(c) => Self::Column(c),
        }
    }
}

impl<V: SQLParam> From<OwnedSQLChunk<V>> for SQLChunk<'static, V> {
    fn from(value: OwnedSQLChunk<V>) -> Self {
        match value {
            OwnedSQLChunk::Token(token) => SQLChunk::Token(token),
            OwnedSQLChunk::Ident(s) => SQLChunk::Ident(Cow::Owned(String::from(s))),
            OwnedSQLChunk::Raw(s) => SQLChunk::Raw(Cow::Owned(String::from(s))),
            OwnedSQLChunk::Number(value) => SQLChunk::Number(value),
            OwnedSQLChunk::Param(param) => SQLChunk::Param(param.into()),
            OwnedSQLChunk::Table(t) => SQLChunk::Table(t),
            OwnedSQLChunk::Column(c) => SQLChunk::Column(c),
        }
    }
}

/// A [`SQL`] fragment that owns all its data, so it can be stored without a
/// lifetime.
///
/// Create one with [`SQL::into_owned`]; turn it back with
/// [`OwnedSQL::into_sql`].
#[derive(Debug, Clone)]
pub struct OwnedSQL<V: SQLParam> {
    /// The fragment's chunks, in order.
    pub chunks: SmallVec<[OwnedSQLChunk<V>; 8]>,
}

impl<V: SQLParam> Default for OwnedSQL<V> {
    fn default() -> Self {
        Self {
            chunks: SmallVec::new(),
        }
    }
}

impl<'a, V: SQLParam> From<SQL<'a, V>> for OwnedSQL<V> {
    fn from(value: SQL<'a, V>) -> Self {
        Self {
            chunks: value.chunks.into_iter().map(Into::into).collect(),
        }
    }
}

impl<V: SQLParam> OwnedSQL<V> {
    /// Creates an empty fragment. Usable in `const` contexts.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            chunks: SmallVec::new_const(),
        }
    }

    /// Creates an empty fragment with room for `capacity` chunks.
    #[inline]
    #[must_use]
    pub fn with_capacity_chunks(capacity: usize) -> Self {
        Self {
            chunks: SmallVec::with_capacity(capacity),
        }
    }

    /// Copies this fragment into a `SQL<'static, V>`.
    pub fn to_sql(&self) -> SQL<'static, V> {
        SQL {
            chunks: self.chunks.iter().cloned().map(Into::into).collect(),
        }
    }

    /// Converts this fragment into a `SQL<'static, V>`.
    pub fn into_sql(self) -> SQL<'static, V> {
        SQL {
            chunks: self.chunks.into_iter().map(Into::into).collect(),
        }
    }
}

impl<V: SQLParam> ToSQL<'static, V> for OwnedSQL<V> {
    fn to_sql(&self) -> SQL<'static, V> {
        Self::to_sql(self)
    }

    fn into_sql(self) -> SQL<'static, V> {
        self.into_sql()
    }
}
