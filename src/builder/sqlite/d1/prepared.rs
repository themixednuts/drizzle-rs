//! D1 prepared statements.
//!
//! Unlike the other SQLite drivers, D1 returns result rows as JSON objects
//! keyed by column name, so the `all`/`get` methods here require
//! `T: serde::Deserialize` instead of `TryFrom<&Row>`. Otherwise the shape
//! mirrors the other SQLite drivers so `drizzle_prepare_impl!()` still works.

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

use ::worker::D1Database;

use super::{bind_statement, sqlite_value_to_js};
use drizzle_core::error::DrizzleError;

/// Convert an iterator of borrowed SQLiteValue-bearing items into the JS param
/// array D1 requires.
fn borrowed_values_to_js<'a, I>(bound: I) -> Vec<wasm_bindgen::JsValue>
where
    I: IntoIterator<Item = SQLiteValue<'a>>,
{
    bound.into_iter().map(|v| sqlite_value_to_js(&v)).collect()
}

/// Same, but for `OwnedSQLiteValue` — used by the `OwnedPreparedStatement`
/// bind paths.
fn owned_values_to_js<I>(bound: I) -> Vec<wasm_bindgen::JsValue>
where
    I: IntoIterator<Item = OwnedSQLiteValue>,
{
    bound
        .into_iter()
        .map(|v| sqlite_value_to_js(&SQLiteValue::from(v)))
        .collect()
}

#[derive(Debug, Clone)]
pub struct PreparedStatement<'a> {
    pub(crate) inner: CorePreparedStatement<'a, SQLiteValue<'a>>,
}

#[derive(Debug, Clone)]
pub struct OwnedPreparedStatement {
    pub(crate) inner: CoreOwnedPreparedStatement<OwnedSQLiteValue>,
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

impl<'a> From<PreparedStatement<'a>> for OwnedPreparedStatement {
    fn from(value: PreparedStatement<'a>) -> Self {
        value.into_owned()
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

    /// Runs the prepared statement and returns the number of affected rows.
    pub async fn execute<const N: usize>(
        &self,
        conn: &D1Database,
        params: [drizzle_core::param::ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> drizzle_core::error::Result<u64> {
        debug_assert_eq!(
            N,
            self.inner.external_param_count(),
            "parameter count mismatch: expected {} params but got {}",
            self.inner.external_param_count(),
            N
        );
        let (sql_str, bound) = self.inner.bind(params)?;
        let values = borrowed_values_to_js(bound);
        let stmt = bind_statement(conn.prepare(sql_str), &values)?;
        run_execute(stmt).await
    }

    /// Runs the prepared statement and returns all matching rows.
    pub async fn all<T, const N: usize>(
        &self,
        conn: &D1Database,
        params: [drizzle_core::param::ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> drizzle_core::error::Result<Vec<T>>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        debug_assert_eq!(
            N,
            self.inner.external_param_count(),
            "parameter count mismatch: expected {} params but got {}",
            self.inner.external_param_count(),
            N
        );
        let (sql_str, bound) = self.inner.bind(params)?;
        let values = borrowed_values_to_js(bound);
        let stmt = bind_statement(conn.prepare(sql_str), &values)?;
        run_all::<T>(stmt).await
    }

    /// Runs the prepared statement and returns a single row.
    pub async fn get<T, const N: usize>(
        &self,
        conn: &D1Database,
        params: [drizzle_core::param::ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> drizzle_core::error::Result<T>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        debug_assert_eq!(
            N,
            self.inner.external_param_count(),
            "parameter count mismatch: expected {} params but got {}",
            self.inner.external_param_count(),
            N
        );
        let (sql_str, bound) = self.inner.bind(params)?;
        let values = borrowed_values_to_js(bound);
        let stmt = bind_statement(conn.prepare(sql_str), &values)?;
        run_get::<T>(stmt).await
    }
}

impl OwnedPreparedStatement {
    /// Runs the prepared statement and returns the number of affected rows.
    pub async fn execute<'a, const N: usize>(
        &self,
        conn: &D1Database,
        params: [drizzle_core::param::ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> drizzle_core::error::Result<u64> {
        debug_assert_eq!(
            N,
            self.inner.external_param_count(),
            "parameter count mismatch: expected {} params but got {}",
            self.inner.external_param_count(),
            N
        );
        let (sql_str, bound) = self.inner.bind(params)?;
        let values = owned_values_to_js(bound);
        let stmt = bind_statement(conn.prepare(sql_str), &values)?;
        run_execute(stmt).await
    }

    /// Runs the prepared statement and returns all matching rows.
    pub async fn all<'a, T, const N: usize>(
        &self,
        conn: &D1Database,
        params: [drizzle_core::param::ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> drizzle_core::error::Result<Vec<T>>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        debug_assert_eq!(
            N,
            self.inner.external_param_count(),
            "parameter count mismatch: expected {} params but got {}",
            self.inner.external_param_count(),
            N
        );
        let (sql_str, bound) = self.inner.bind(params)?;
        let values = owned_values_to_js(bound);
        let stmt = bind_statement(conn.prepare(sql_str), &values)?;
        run_all::<T>(stmt).await
    }

    /// Runs the prepared statement and returns a single row.
    pub async fn get<'a, T, const N: usize>(
        &self,
        conn: &D1Database,
        params: [drizzle_core::param::ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> drizzle_core::error::Result<T>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        debug_assert_eq!(
            N,
            self.inner.external_param_count(),
            "parameter count mismatch: expected {} params but got {}",
            self.inner.external_param_count(),
            N
        );
        let (sql_str, bound) = self.inner.bind(params)?;
        let values = owned_values_to_js(bound);
        let stmt = bind_statement(conn.prepare(sql_str), &values)?;
        run_get::<T>(stmt).await
    }
}

async fn run_execute(stmt: ::worker::D1PreparedStatement) -> drizzle_core::error::Result<u64> {
    let result = stmt
        .run()
        .await
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    if !result.success() {
        return Err(DrizzleError::Other(
            result
                .error()
                .unwrap_or_else(|| "D1 statement failed".into())
                .into(),
        ));
    }
    Ok(result
        .meta()
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?
        .and_then(|m| m.changes)
        .unwrap_or(0) as u64)
}

async fn run_all<T>(stmt: ::worker::D1PreparedStatement) -> drizzle_core::error::Result<Vec<T>>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let result = stmt
        .all()
        .await
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    if !result.success() {
        return Err(DrizzleError::Other(
            result
                .error()
                .unwrap_or_else(|| "D1 query failed".into())
                .into(),
        ));
    }
    result
        .results::<T>()
        .map_err(|e| DrizzleError::Other(e.to_string().into()))
}

async fn run_get<T>(stmt: ::worker::D1PreparedStatement) -> drizzle_core::error::Result<T>
where
    T: for<'de> serde::Deserialize<'de>,
{
    stmt.first::<T>(None)
        .await
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?
        .ok_or(DrizzleError::NotFound)
}

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
