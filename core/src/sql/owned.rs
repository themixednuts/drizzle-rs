use crate::prelude::*;
use crate::{OwnedParam, SQL, SQLChunk, SQLColumnInfo, SQLParam, SQLTableInfo, ToSQL, Token};
use smallvec::SmallVec;

/// Owned version of SQLChunk with 'static lifetime
#[derive(Debug, Clone)]
pub enum OwnedSQLChunk<V: SQLParam> {
    Token(Token),
    Ident(String),
    Raw(String),
    Number(usize),
    Param(OwnedParam<V>),
    Table(&'static dyn SQLTableInfo),
    Column(&'static dyn SQLColumnInfo),
}

impl<V: SQLParam> OwnedSQLChunk<V> {
    /// Creates a token chunk.
    #[inline]
    pub const fn token(t: Token) -> Self {
        Self::Token(t)
    }

    /// Creates a table chunk.
    #[inline]
    pub const fn table(table: &'static dyn SQLTableInfo) -> Self {
        Self::Table(table)
    }

    /// Creates a column chunk.
    #[inline]
    pub const fn column(column: &'static dyn SQLColumnInfo) -> Self {
        Self::Column(column)
    }

    /// Creates a quoted identifier from a runtime string.
    #[inline]
    pub fn ident(name: impl Into<String>) -> Self {
        Self::Ident(name.into())
    }

    /// Creates raw SQL text from a runtime string.
    #[inline]
    pub fn raw(text: impl Into<String>) -> Self {
        Self::Raw(text.into())
    }

    /// Creates a parameter chunk with owned value.
    #[inline]
    pub fn param(value: OwnedParam<V>) -> Self {
        Self::Param(value)
    }
}

impl<'a, V: SQLParam> From<SQLChunk<'a, V>> for OwnedSQLChunk<V> {
    fn from(value: SQLChunk<'a, V>) -> Self {
        match value {
            SQLChunk::Token(token) => Self::Token(token),
            SQLChunk::Ident(cow) => Self::Ident(cow.into_owned()),
            SQLChunk::Raw(cow) => Self::Raw(cow.into_owned()),
            SQLChunk::Number(value) => Self::Number(value),
            SQLChunk::Param(param) => Self::Param(param.into()),
            SQLChunk::Table(table) => Self::Table(table),
            SQLChunk::Column(column) => Self::Column(column),
        }
    }
}

impl<V: SQLParam> From<OwnedSQLChunk<V>> for SQLChunk<'static, V> {
    fn from(value: OwnedSQLChunk<V>) -> Self {
        match value {
            OwnedSQLChunk::Token(token) => SQLChunk::Token(token),
            OwnedSQLChunk::Ident(s) => SQLChunk::Ident(Cow::Owned(s)),
            OwnedSQLChunk::Raw(s) => SQLChunk::Raw(Cow::Owned(s)),
            OwnedSQLChunk::Number(value) => SQLChunk::Number(value),
            OwnedSQLChunk::Param(param) => SQLChunk::Param(param.into()),
            OwnedSQLChunk::Table(table) => SQLChunk::Table(table),
            OwnedSQLChunk::Column(column) => SQLChunk::Column(column),
        }
    }
}

/// Owned version of SQL with 'static lifetime
#[derive(Debug, Clone)]
pub struct OwnedSQL<V: SQLParam> {
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
    /// Creates an empty SQL fragment with const-friendly initialization.
    #[inline]
    pub const fn empty() -> Self {
        Self {
            chunks: SmallVec::new_const(),
        }
    }

    /// Creates an empty SQL fragment with pre-allocated chunk capacity.
    #[inline]
    pub fn with_capacity_chunks(capacity: usize) -> Self {
        Self {
            chunks: SmallVec::with_capacity(capacity),
        }
    }

    /// Convert to SQL with 'static lifetime
    pub fn to_sql(&self) -> SQL<'static, V> {
        SQL {
            chunks: self.chunks.iter().cloned().map(Into::into).collect(),
        }
    }

    /// Convert into SQL with 'static lifetime (consuming)
    pub fn into_sql(self) -> SQL<'static, V> {
        SQL {
            chunks: self.chunks.into_iter().map(Into::into).collect(),
        }
    }
}

impl<V: SQLParam> ToSQL<'static, V> for OwnedSQL<V> {
    fn to_sql(&self) -> SQL<'static, V> {
        OwnedSQL::to_sql(self)
    }

    fn into_sql(self) -> SQL<'static, V> {
        self.into_sql()
    }
}
