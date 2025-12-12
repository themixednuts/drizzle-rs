use crate::prelude::*;
use crate::{OwnedParam, SQL, SQLChunk, SQLColumnInfo, SQLParam, SQLTableInfo, ToSQL, Token};
use smallvec::SmallVec;

/// Owned version of SQLChunk with 'static lifetime
#[derive(Debug, Clone)]
pub enum OwnedSQLChunk<V: SQLParam> {
    Token(Token),
    Ident(String),
    Raw(String),
    Param(OwnedParam<V>),
    Table(&'static dyn SQLTableInfo),
    Column(&'static dyn SQLColumnInfo),
    Alias {
        inner: Box<OwnedSQLChunk<V>>,
        alias: String,
    },
}

impl<'a, V: SQLParam> From<SQLChunk<'a, V>> for OwnedSQLChunk<V> {
    fn from(value: SQLChunk<'a, V>) -> Self {
        match value {
            SQLChunk::Token(token) => Self::Token(token),
            SQLChunk::Ident(cow) => Self::Ident(cow.into_owned()),
            SQLChunk::Raw(cow) => Self::Raw(cow.into_owned()),
            SQLChunk::Param(param) => Self::Param(param.into()),
            SQLChunk::Table(table) => Self::Table(table),
            SQLChunk::Column(column) => Self::Column(column),
            SQLChunk::Alias { inner, alias } => Self::Alias {
                inner: Box::new((*inner).into()),
                alias: alias.into_owned(),
            },
        }
    }
}

impl<V: SQLParam> From<OwnedSQLChunk<V>> for SQLChunk<'static, V> {
    fn from(value: OwnedSQLChunk<V>) -> Self {
        match value {
            OwnedSQLChunk::Token(token) => SQLChunk::Token(token),
            OwnedSQLChunk::Ident(s) => SQLChunk::Ident(Cow::Owned(s)),
            OwnedSQLChunk::Raw(s) => SQLChunk::Raw(Cow::Owned(s)),
            OwnedSQLChunk::Param(param) => SQLChunk::Param(param.into()),
            OwnedSQLChunk::Table(table) => SQLChunk::Table(table),
            OwnedSQLChunk::Column(column) => SQLChunk::Column(column),
            OwnedSQLChunk::Alias { inner, alias } => SQLChunk::Alias {
                inner: Box::new((*inner).into()),
                alias: Cow::Owned(alias),
            },
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
}
