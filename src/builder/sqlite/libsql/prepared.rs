use drizzle_core::{
    param::{OwnedParam, Param},
    prepared::{
        OwnedPreparedStatement as CoreOwnedPreparedStatement,
        PreparedStatement as CorePreparedStatement,
    },
    traits::ToSQL,
};
use drizzle_sqlite::values::{OwnedSQLiteValue, SQLiteValue};
use std::borrow::Cow;

use libsql::{Connection, Row};

use super::super::prepared_common::sqlite_async_prepared_impl;

/// Trait for types that can execute SQL queries asynchronously.
///
/// Both [`libsql::Connection`] and [`libsql::Transaction`] implement this trait,
/// allowing prepared statements to be used with either.
pub trait LibsqlExecutor {
    fn fetch(
        &self,
        sql: &str,
        params: Vec<libsql::Value>,
    ) -> impl std::future::Future<Output = drizzle_core::error::Result<libsql::Rows>>;

    fn exec(
        &self,
        sql: &str,
        params: Vec<libsql::Value>,
    ) -> impl std::future::Future<Output = drizzle_core::error::Result<u64>>;
}

impl LibsqlExecutor for Connection {
    async fn fetch(
        &self,
        sql: &str,
        params: Vec<libsql::Value>,
    ) -> drizzle_core::error::Result<libsql::Rows> {
        Ok(self.query(sql, params).await?)
    }

    async fn exec(
        &self,
        sql: &str,
        params: Vec<libsql::Value>,
    ) -> drizzle_core::error::Result<u64> {
        self.execute(sql, params).await.map_err(Into::into)
    }
}

impl LibsqlExecutor for libsql::Transaction {
    async fn fetch(
        &self,
        sql: &str,
        params: Vec<libsql::Value>,
    ) -> drizzle_core::error::Result<libsql::Rows> {
        Ok(self.query(sql, params).await?)
    }

    async fn exec(
        &self,
        sql: &str,
        params: Vec<libsql::Value>,
    ) -> drizzle_core::error::Result<u64> {
        self.execute(sql, params).await.map_err(Into::into)
    }
}

#[derive(Debug, Clone)]
pub struct PreparedStatement<'a> {
    pub(crate) inner: CorePreparedStatement<'a, SQLiteValue<'a>>,
}

impl From<OwnedPreparedStatement> for PreparedStatement<'_> {
    fn from(value: OwnedPreparedStatement) -> Self {
        let sqlitevalue = value.inner.params.iter().map(|v| {
            Param::new(
                v.placeholder,
                v.value.clone().map(|v| Cow::Owned(SQLiteValue::from(v))),
            )
        });
        let inner = CorePreparedStatement {
            text_segments: value.inner.text_segments,
            params: sqlitevalue.collect::<Box<[_]>>(),
            sql: value.inner.sql,
        };
        PreparedStatement { inner }
    }
}

impl<'a> PreparedStatement<'a> {
    pub fn into_owned(self) -> OwnedPreparedStatement {
        let owned_params = self.inner.params.iter().map(|p| OwnedParam {
            placeholder: p.placeholder,
            value: p
                .value
                .clone()
                .map(|v| OwnedSQLiteValue::from(v.into_owned())),
        });

        let inner = CoreOwnedPreparedStatement {
            text_segments: self.inner.text_segments.clone(),
            params: owned_params.collect::<Box<[_]>>(),
            sql: self.inner.sql.clone(),
        };

        OwnedPreparedStatement { inner }
    }
}

#[derive(Debug, Clone)]
pub struct OwnedPreparedStatement {
    pub(crate) inner: CoreOwnedPreparedStatement<OwnedSQLiteValue>,
}

impl<'a> From<PreparedStatement<'a>> for OwnedPreparedStatement {
    fn from(value: PreparedStatement<'a>) -> Self {
        value.into_owned()
    }
}

sqlite_async_prepared_impl!(LibsqlExecutor, Row, libsql::Value);

impl<'a> std::fmt::Display for PreparedStatement<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl std::fmt::Display for OwnedPreparedStatement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl<'a> ToSQL<'a, SQLiteValue<'a>> for PreparedStatement<'a> {
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, SQLiteValue<'a>> {
        self.inner.to_sql()
    }
}

impl<'a> ToSQL<'a, OwnedSQLiteValue> for OwnedPreparedStatement {
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, OwnedSQLiteValue> {
        self.inner.to_sql()
    }
}

impl<'a> ToSQL<'a, SQLiteValue<'a>> for OwnedPreparedStatement {
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, SQLiteValue<'a>> {
        self.inner.to_sql().map_params(SQLiteValue::from)
    }
}
