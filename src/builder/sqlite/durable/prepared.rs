//! Prepared statements for the Durable Objects SQL driver.
//!
//! Rows come back as column-keyed objects, so `all`/`get` require
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
use std::{borrow::Cow, marker::PhantomData};

use ::worker::{SqlStorage, SqlStorageValue};

use super::sqlite_value_to_storage;
use drizzle_core::error::DrizzleError;

/// Convert an iterator of borrowed SQLiteValue-bearing items into the typed
/// [`SqlStorageValue`] vector DO SQL requires.
fn borrowed_values_to_storage<'a, I>(bound: I) -> Vec<SqlStorageValue>
where
    I: IntoIterator<Item = SQLiteValue<'a>>,
{
    bound
        .into_iter()
        .map(|v| sqlite_value_to_storage(&v))
        .collect()
}

/// Same, but for `OwnedSQLiteValue` — used by the `OwnedPreparedStatement`
/// bind paths.
fn owned_values_to_storage<I>(bound: I) -> Vec<SqlStorageValue>
where
    I: IntoIterator<Item = OwnedSQLiteValue>,
{
    bound
        .into_iter()
        .map(|v| sqlite_value_to_storage(&SQLiteValue::from(v)))
        .collect()
}

#[derive(Debug, Clone)]
pub struct PreparedStatement<'a, Marker = (), DecodedRow = ()> {
    pub(crate) inner: CorePreparedStatement<'a, SQLiteValue<'a>>,
    pub(crate) marker: PhantomData<(Marker, DecodedRow)>,
}

#[derive(Debug, Clone)]
pub struct OwnedPreparedStatement<Marker = (), DecodedRow = ()> {
    pub(crate) inner: CoreOwnedPreparedStatement<OwnedSQLiteValue>,
    pub(crate) marker: PhantomData<(Marker, DecodedRow)>,
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

impl<'a, Marker, DecodedRow> From<PreparedStatement<'a, Marker, DecodedRow>>
    for OwnedPreparedStatement<Marker, DecodedRow>
{
    fn from(value: PreparedStatement<'a, Marker, DecodedRow>) -> Self {
        value.into_owned()
    }
}

impl<'a, Marker, DecodedRow> PreparedStatement<'a, Marker, DecodedRow> {
    pub(crate) fn new(inner: CorePreparedStatement<'a, SQLiteValue<'a>>) -> Self {
        Self {
            inner,
            marker: PhantomData,
        }
    }

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
        let values = borrowed_values_to_storage(bound);
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
        let values = borrowed_values_to_storage(bound);
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
        let values = borrowed_values_to_storage(bound);
        run_get::<T>(conn, sql_str, values)
    }
}

impl<Marker, DecodedRow> OwnedPreparedStatement<Marker, DecodedRow> {
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
        let values = owned_values_to_storage(bound);
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
        let values = owned_values_to_storage(bound);
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
        let values = owned_values_to_storage(bound);
        run_get::<T>(conn, sql_str, values)
    }
}

fn run_execute(
    conn: &SqlStorage,
    sql: &str,
    values: Vec<SqlStorageValue>,
) -> drizzle_core::error::Result<u64> {
    let cursor = conn
        .exec(sql, Some(values))
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    // Drain so `rows_written` is populated.
    let _ = cursor
        .to_array::<serde::de::IgnoredAny>()
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    Ok(cursor.rows_written() as u64)
}

fn run_all<T>(
    conn: &SqlStorage,
    sql: &str,
    values: Vec<SqlStorageValue>,
) -> drizzle_core::error::Result<Vec<T>>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let cursor = conn
        .exec(sql, Some(values))
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    cursor
        .to_array::<T>()
        .map_err(|e| DrizzleError::Other(e.to_string().into()))
}

fn run_get<T>(
    conn: &SqlStorage,
    sql: &str,
    values: Vec<SqlStorageValue>,
) -> drizzle_core::error::Result<T>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let cursor = conn
        .exec(sql, Some(values))
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
    cursor
        .to_array::<T>()
        .map_err(|e| DrizzleError::Other(e.to_string().into()))?
        .into_iter()
        .next()
        .ok_or(DrizzleError::NotFound)
}

impl<'a, Marker, DecodedRow> std::fmt::Display for PreparedStatement<'a, Marker, DecodedRow> {
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
