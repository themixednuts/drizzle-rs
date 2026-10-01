use drizzle_core::{
    param::{OwnedParam, Param},
    prepared::{
        OwnedPreparedStatement as CoreOwnedPreparedStatement,
        PreparedStatement as CorePreparedStatement,
    },
    traits::ToSQL,
};
use drizzle_sqlite::values::{OwnedSQLiteValue, SQLiteValue};
use std::{borrow::Cow, marker::PhantomData};

use turso::{Connection, Row};

use super::super::prepared_common::sqlite_async_prepared_impl;

/// A turso connection or transaction that a prepared statement can run on.
///
/// Implemented for [`turso::Connection`] and [`turso::transaction::Transaction`], so the same
/// prepared statement runs inside or outside a transaction.
pub trait TursoExecutor {
    /// Runs `sql` with `params` and returns its rows.
    fn fetch(
        &self,
        sql: &str,
        params: Vec<turso::Value>,
    ) -> impl std::future::Future<Output = drizzle_core::error::Result<turso::Rows>>;

    /// Runs `sql` with `params` and returns the number of rows it changed.
    fn exec(
        &self,
        sql: &str,
        params: Vec<turso::Value>,
    ) -> impl std::future::Future<Output = drizzle_core::error::Result<u64>>;
}

impl TursoExecutor for Connection {
    async fn fetch(
        &self,
        sql: &str,
        params: Vec<turso::Value>,
    ) -> drizzle_core::error::Result<turso::Rows> {
        self.execute_batch("").await?;
        let mut stmt = self.prepare_cached(sql).await?;
        Ok(stmt.query(params).await?)
    }

    async fn exec(&self, sql: &str, params: Vec<turso::Value>) -> drizzle_core::error::Result<u64> {
        self.execute_batch("").await?;
        let mut stmt = self.prepare_cached(sql).await?;
        super::run_statement(&mut stmt, params)
            .await
            .map_err(Into::into)
    }
}

impl TursoExecutor for turso::transaction::Transaction<'_> {
    async fn fetch(
        &self,
        sql: &str,
        params: Vec<turso::Value>,
    ) -> drizzle_core::error::Result<turso::Rows> {
        let mut stmt = self.prepare_cached(sql).await?;
        Ok(stmt.query(params).await?)
    }

    async fn exec(&self, sql: &str, params: Vec<turso::Value>) -> drizzle_core::error::Result<u64> {
        let mut stmt = self.prepare_cached(sql).await?;
        super::run_statement(&mut stmt, params)
            .await
            .map_err(Into::into)
    }
}

/// A query rendered once, ready to run many times with new placeholder values.
///
/// Made by `.prepare()` on a query builder. It borrows the values embedded in
/// the query; call [`into_owned`](Self::into_owned) to store it.
#[derive(Debug, Clone)]
pub struct PreparedStatement<'a, Marker = (), DecodedRow = ()> {
    pub(crate) inner: CorePreparedStatement<'a, SQLiteValue<'a>>,
    pub(crate) marker: PhantomData<(Marker, DecodedRow)>,
}

impl<'a, Marker, DecodedRow> PreparedStatement<'a, Marker, DecodedRow> {
    pub(crate) fn new(inner: CorePreparedStatement<'a, SQLiteValue<'a>>) -> Self {
        Self {
            inner,
            marker: PhantomData,
        }
    }

    /// Copies the embedded values so the statement no longer borrows them.
    pub fn into_owned(self) -> OwnedPreparedStatement<Marker, DecodedRow> {
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

        OwnedPreparedStatement {
            inner,
            marker: PhantomData,
        }
    }
}

/// A [`PreparedStatement`] that owns its embedded values, so it can be stored
/// or moved freely.
#[derive(Debug, Clone)]
pub struct OwnedPreparedStatement<Marker = (), DecodedRow = ()> {
    pub(crate) inner: CoreOwnedPreparedStatement<OwnedSQLiteValue>,
    pub(crate) marker: PhantomData<(Marker, DecodedRow)>,
}

impl<'a, Marker, DecodedRow> From<PreparedStatement<'a, Marker, DecodedRow>>
    for OwnedPreparedStatement<Marker, DecodedRow>
{
    fn from(value: PreparedStatement<'a, Marker, DecodedRow>) -> Self {
        value.into_owned()
    }
}

impl<Marker, DecodedRow> From<OwnedPreparedStatement<Marker, DecodedRow>>
    for PreparedStatement<'_, Marker, DecodedRow>
{
    fn from(value: OwnedPreparedStatement<Marker, DecodedRow>) -> Self {
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
        PreparedStatement {
            inner,
            marker: PhantomData,
        }
    }
}

sqlite_async_prepared_impl!(TursoExecutor, Row, turso::Value);

impl<Marker, DecodedRow> std::fmt::Display for PreparedStatement<'_, Marker, DecodedRow> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl<Marker, DecodedRow> std::fmt::Display for OwnedPreparedStatement<Marker, DecodedRow> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}
impl<'a, Marker, DecodedRow> ToSQL<'a, SQLiteValue<'a>>
    for PreparedStatement<'a, Marker, DecodedRow>
{
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, SQLiteValue<'a>> {
        self.inner.to_sql()
    }
}

impl<'a, Marker, DecodedRow> ToSQL<'a, OwnedSQLiteValue>
    for OwnedPreparedStatement<Marker, DecodedRow>
{
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, OwnedSQLiteValue> {
        self.inner.to_sql()
    }
}

impl<'a, Marker, DecodedRow> ToSQL<'a, SQLiteValue<'a>>
    for OwnedPreparedStatement<Marker, DecodedRow>
{
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, SQLiteValue<'a>> {
        self.inner.to_sql().map_params(SQLiteValue::from)
    }
}
