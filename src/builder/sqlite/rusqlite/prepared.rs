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

/// A query rendered once, ready to run many times with new placeholder values.
///
/// Made by `.prepare()` on a query builder. It borrows the values embedded in
/// the query; call [`into_owned`](Self::into_owned) to store it.
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

    /// Binds `params` and runs the statement, returning the number of rows it
    /// changed.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ParameterError`] when `params` do not match the
    /// statement's placeholders (missing, duplicated, or extra),
    /// or the database error when the statement fails.
    ///
    /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError
    ///
    /// # Panics
    ///
    /// In debug builds, panics when `N` differs from the number of placeholders.
    pub fn execute<const N: usize>(
        &self,
        conn: &Connection,
        params: [ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> Result<usize> {
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
        super::run_statement(&mut stmt, params_from_iter(params)).map_err(Into::into)
    }

    /// Binds `params`, runs the query, and decodes every row into `T`.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ParameterError`] when `params` do not match the
    /// statement's placeholders (missing, duplicated, or extra),
    /// the database error when the query fails, or a decode error when a row
    /// does not fit `T`.
    ///
    /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError
    ///
    /// # Panics
    ///
    /// In debug builds, panics when `N` differs from the number of placeholders.
    pub fn all<T, const N: usize>(
        &self,
        conn: &Connection,
        params: [ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> Result<Vec<T>>
    where
        for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r Row<'r>, T>,
    {
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

    /// Binds `params`, runs the query, and decodes its first row into `T`.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ParameterError`] when `params` do not match the
    /// statement's placeholders (missing, duplicated, or extra),
    /// rusqlite's `QueryReturnedNoRows` when no row matches, the database
    /// error when the query fails, or a decode error when the row does not fit `T`.
    ///
    /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError
    ///
    /// # Panics
    ///
    /// In debug builds, panics when `N` differs from the number of placeholders.
    pub fn get<T, const N: usize>(
        &self,
        conn: &Connection,
        params: [ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> Result<T>
    where
        for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r Row<'r>, T>,
    {
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

impl<Marker, DecodedRow> OwnedPreparedStatement<Marker, DecodedRow> {
    /// Binds `params` and runs the statement, returning the number of rows it
    /// changed.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ParameterError`] when `params` do not match the
    /// statement's placeholders (missing, duplicated, or extra),
    /// or the database error when the statement fails.
    ///
    /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError
    ///
    /// # Panics
    ///
    /// In debug builds, panics when `N` differs from the number of placeholders.
    pub fn execute<'a, const N: usize>(
        &self,
        conn: &Connection,
        params: [ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> Result<usize> {
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
        Ok(super::run_statement(&mut stmt, params_from_iter(params))?)
    }

    /// Binds `params`, runs the query, and decodes every row into `T`.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ParameterError`] when `params` do not match the
    /// statement's placeholders (missing, duplicated, or extra),
    /// the database error when the query fails, or a decode error when a row
    /// does not fit `T`.
    ///
    /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError
    ///
    /// # Panics
    ///
    /// In debug builds, panics when `N` differs from the number of placeholders.
    pub fn all<'a, T, const N: usize>(
        &self,
        conn: &Connection,
        params: [ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> Result<Vec<T>>
    where
        for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r Row<'r>, T>,
    {
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

    /// Binds `params`, runs the query, and decodes its first row into `T`.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ParameterError`] when `params` do not match the
    /// statement's placeholders (missing, duplicated, or extra),
    /// rusqlite's `QueryReturnedNoRows` when no row matches, the database
    /// error when the query fails, or a decode error when the row does not fit `T`.
    ///
    /// [`DrizzleError::ParameterError`]: drizzle_core::error::DrizzleError::ParameterError
    ///
    /// # Panics
    ///
    /// In debug builds, panics when `N` differs from the number of placeholders.
    pub fn get<'a, T, const N: usize>(
        &self,
        conn: &Connection,
        params: [ParamBind<'a, SQLiteValue<'a>>; N],
    ) -> Result<T>
    where
        for<'r> Marker: drizzle_core::row::DecodeSelectedRef<&'r Row<'r>, T>,
    {
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
