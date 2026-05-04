use drizzle_core::error::Result;
use drizzle_core::param::{OwnedParam, Param, ParamBind};
use drizzle_core::prepared::{
    OwnedPreparedStatement as CoreOwnedPreparedStatement,
    PreparedStatement as CorePreparedStatement,
};
use drizzle_core::traits::ToSQL;
use drizzle_sqlite::values::{OwnedSQLiteValue, SQLiteValue};
use std::{borrow::Cow, marker::PhantomData};

use rusqlite::{Connection, Row, params_from_iter};

#[derive(Debug, Clone)]
pub struct PreparedStatement<'a, Marker = (), DecodedRow = ()> {
    pub(crate) inner: CorePreparedStatement<'a, SQLiteValue<'a>>,
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

impl<'a, Marker, DecodedRow> PreparedStatement<'a, Marker, DecodedRow> {
    pub(crate) fn new(inner: CorePreparedStatement<'a, SQLiteValue<'a>>) -> Self {
        Self {
            inner,
            marker: PhantomData,
        }
    }

    /// Runs the prepared statement and returns the number of affected rows
    pub fn execute<const N: usize>(
        &self,
        conn: &Connection,
        params: [ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> Result<usize> {
        debug_assert_eq!(
            N,
            self.inner.external_param_count(),
            "parameter count mismatch: expected {} params but got {}",
            self.inner.external_param_count(),
            N
        );
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "prepared.execute");
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "prepared.execute.bind");
            self.inner.bind(params)?
        };

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "prepared.execute.db");
        let mut stmt = conn.prepare_cached(sql_str)?;
        stmt.execute(params_from_iter(params)).map_err(Into::into)
    }

    /// Runs the prepared statement and returns all matching rows
    pub fn all<T, const N: usize>(
        &self,
        conn: &Connection,
        params: [ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> Result<Vec<T>>
    where
        for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r Row<'r>, T>,
    {
        debug_assert_eq!(
            N,
            self.inner.external_param_count(),
            "parameter count mismatch: expected {} params but got {}",
            self.inner.external_param_count(),
            N
        );
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "prepared.all");
        let (sql_str, params) = self.inner.bind(params)?;

        let mut stmt = conn.prepare_cached(sql_str)?;

        let mut rows = stmt.query_and_then(params_from_iter(params), |row| {
            <Marker as drizzle_core::row::DecodeSelectedRef<&Row<'_>, T>>::decode(row)
        })?;

        let (lower, _) = rows.size_hint();
        let mut results = Vec::with_capacity(lower);
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    /// Runs the prepared statement and returns a single row
    pub fn get<T, const N: usize>(
        &self,
        conn: &Connection,
        params: [ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> Result<T>
    where
        for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r Row<'r>, T>,
    {
        debug_assert_eq!(
            N,
            self.inner.external_param_count(),
            "parameter count mismatch: expected {} params but got {}",
            self.inner.external_param_count(),
            N
        );
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "prepared.get");
        let (sql_str, params) = self.inner.bind(params)?;

        let mut stmt = conn.prepare_cached(sql_str)?;

        stmt.query_row(params_from_iter(params), |row| {
            Ok(<Marker as drizzle_core::row::DecodeSelectedRef<
                &Row<'_>,
                T,
            >>::decode(row))
        })?
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
}

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

impl<Marker, DecodedRow> OwnedPreparedStatement<Marker, DecodedRow> {
    /// Runs the prepared statement and returns the number of affected rows
    pub fn execute<'a, const N: usize>(
        &self,
        conn: &Connection,
        params: [ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> Result<usize> {
        debug_assert_eq!(
            N,
            self.inner.external_param_count(),
            "parameter count mismatch: expected {} params but got {}",
            self.inner.external_param_count(),
            N
        );
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "owned_prepared.execute");
        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "owned_prepared.execute.bind");
            self.inner.bind(params)?
        };

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "owned_prepared.execute.db");
        let mut stmt = conn.prepare_cached(sql_str)?;
        Ok(stmt.execute(params_from_iter(params))?)
    }

    /// Runs the prepared statement and returns all matching rows
    pub fn all<'a, T, const N: usize>(
        &self,
        conn: &Connection,
        params: [ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> Result<Vec<T>>
    where
        for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r Row<'r>, T>,
    {
        debug_assert_eq!(
            N,
            self.inner.external_param_count(),
            "parameter count mismatch: expected {} params but got {}",
            self.inner.external_param_count(),
            N
        );
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "owned_prepared.all");
        let (sql_str, params) = self.inner.bind(params)?;

        let mut stmt = conn.prepare_cached(sql_str)?;

        let mut rows = stmt.query_and_then(params_from_iter(params), |row| {
            <Marker as drizzle_core::row::DecodeSelectedRef<&Row<'_>, T>>::decode(row)
        })?;

        let (lower, _) = rows.size_hint();
        let mut results = Vec::with_capacity(lower);
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    /// Runs the prepared statement and returns a single row
    pub fn get<'a, T, const N: usize>(
        &self,
        conn: &Connection,
        params: [ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> Result<T>
    where
        for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r Row<'r>, T>,
    {
        debug_assert_eq!(
            N,
            self.inner.external_param_count(),
            "parameter count mismatch: expected {} params but got {}",
            self.inner.external_param_count(),
            N
        );
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "owned_prepared.get");
        let (sql_str, params) = self.inner.bind(params)?;

        let mut stmt = conn.prepare_cached(sql_str)?;

        stmt.query_row(params_from_iter(params), |row| {
            Ok(<Marker as drizzle_core::row::DecodeSelectedRef<
                &Row<'_>,
                T,
            >>::decode(row))
        })?
    }
}

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
