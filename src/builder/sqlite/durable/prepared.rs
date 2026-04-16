//! Prepared statements for the Durable Objects SQL driver.
//!
//! Mirrors the shape of the other SQLite drivers (`drizzle_prepare_impl!`
//! produces the outer `PreparedStatement` / `OwnedPreparedStatement` wrappers)
//! but dispatches to synchronous `SqlStorage::exec_raw`. Like D1, rows come
//! back as column-keyed JS objects, so `all`/`get` require
//! `T: serde::Deserialize`.

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

use ::worker::SqlStorage;

use super::sqlite_value_to_js;
use drizzle_core::error::DrizzleError;

/// Convert an iterator of borrowed SQLiteValue-bearing items into the JS param
/// vector DO SQL requires.
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

    /// Runs the prepared statement and returns the number of rows written.
    pub fn execute<const N: usize>(
        &self,
        conn: &SqlStorage,
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
        run_execute(conn, sql_str, values)
    }

    /// Runs the prepared statement and returns all matching rows.
    pub fn all<T, const N: usize>(
        &self,
        conn: &SqlStorage,
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
        run_all::<T>(conn, sql_str, values)
    }

    /// Runs the prepared statement and returns a single row.
    pub fn get<T, const N: usize>(
        &self,
        conn: &SqlStorage,
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
        run_get::<T>(conn, sql_str, values)
    }
}

impl OwnedPreparedStatement {
    /// Runs the prepared statement and returns the number of rows written.
    pub fn execute<'a, const N: usize>(
        &self,
        conn: &SqlStorage,
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
        run_execute(conn, sql_str, values)
    }

    /// Runs the prepared statement and returns all matching rows.
    pub fn all<'a, T, const N: usize>(
        &self,
        conn: &SqlStorage,
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
        run_all::<T>(conn, sql_str, values)
    }

    /// Runs the prepared statement and returns a single row.
    pub fn get<'a, T, const N: usize>(
        &self,
        conn: &SqlStorage,
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
        run_get::<T>(conn, sql_str, values)
    }
}

fn run_execute(
    conn: &SqlStorage,
    sql: &str,
    values: Vec<wasm_bindgen::JsValue>,
) -> drizzle_core::error::Result<u64> {
    let cursor = conn
        .exec_raw(sql, values)
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    // Drain so `rows_written` is populated.
    let _ = cursor
        .to_array::<serde_json::Value>()
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    Ok(cursor.rows_written() as u64)
}

fn run_all<T>(
    conn: &SqlStorage,
    sql: &str,
    values: Vec<wasm_bindgen::JsValue>,
) -> drizzle_core::error::Result<Vec<T>>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let cursor = conn
        .exec_raw(sql, values)
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    cursor
        .to_array::<T>()
        .map_err(|e| DrizzleError::Other(e.to_string().into()))
}

fn run_get<T>(
    conn: &SqlStorage,
    sql: &str,
    values: Vec<wasm_bindgen::JsValue>,
) -> drizzle_core::error::Result<T>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let cursor = conn
        .exec_raw(sql, values)
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    cursor
        .to_array::<T>()
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?
        .into_iter()
        .next()
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
