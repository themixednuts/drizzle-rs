use drizzle_core::error::{DrizzleError, Result};
use drizzle_core::param::{OwnedParam, Param, ParamBind};
use drizzle_core::prepared::{
    OwnedPreparedStatement as CoreOwnedPreparedStatement,
    PreparedStatement as CorePreparedStatement,
};
use drizzle_core::traits::ToSQL;
use drizzle_sqlite::values::{OwnedSQLiteValue, SQLiteValue};
use std::borrow::Cow;

use rusqlite::{Connection, Row, params_from_iter};

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
    #[inline]
    fn has_all_internal_params(&self) -> bool {
        self.inner.params.iter().all(|p| p.value.is_some())
    }

    /// Runs the prepared statement and returns the number of affected rows
    pub fn execute(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = ParamBind<'a, SQLiteValue<'a>>>,
    ) -> Result<usize> {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "prepared.execute");
        if self.has_all_internal_params() {
            let params = self
                .inner
                .params
                .iter()
                .filter_map(|p| p.value.as_ref().map(|v| v.as_ref()));

            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "prepared.execute.db");
            return conn
                .execute(self.inner.sql(), params_from_iter(params))
                .map_err(Into::into);
        }

        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "prepared.execute.bind");
            self.inner.bind(params)
        };

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "prepared.execute.db");
        conn.execute(sql_str, params_from_iter(params))
            .map_err(Into::into)
    }

    /// Runs the prepared statement and returns all matching rows
    pub fn all<T>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = ParamBind<'a, SQLiteValue<'a>>>,
    ) -> Result<Vec<T>>
    where
        T: for<'r> TryFrom<&'r Row<'r>>,
        for<'r> <T as TryFrom<&'r Row<'r>>>::Error: Into<DrizzleError>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "prepared.all");
        if self.has_all_internal_params() {
            let mut stmt = conn.prepare(self.inner.sql())?;
            let params = self
                .inner
                .params
                .iter()
                .filter_map(|p| p.value.as_ref().map(|v| v.as_ref()));

            let mut rows = stmt.query_and_then(params_from_iter(params), |row| {
                T::try_from(row).map_err(Into::into)
            })?;

            let (lower, _) = rows.size_hint();
            let mut results = Vec::with_capacity(lower);
            for row in rows {
                results.push(row?);
            }

            return Ok(results);
        }

        let (sql_str, params) = self.inner.bind(params);

        let mut stmt = conn.prepare(sql_str)?;

        let mut rows = stmt.query_and_then(params_from_iter(params), |row| {
            T::try_from(row).map_err(Into::into)
        })?;

        let (lower, _) = rows.size_hint();
        let mut results = Vec::with_capacity(lower);
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    /// Runs the prepared statement and returns a single row
    pub fn get<T>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = ParamBind<'a, SQLiteValue<'a>>>,
    ) -> Result<T>
    where
        T: for<'r> TryFrom<&'r Row<'r>>,
        for<'r> <T as TryFrom<&'r Row<'r>>>::Error: Into<DrizzleError>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "prepared.get");
        if self.has_all_internal_params() {
            let mut stmt = conn.prepare(self.inner.sql())?;
            let params = self
                .inner
                .params
                .iter()
                .filter_map(|p| p.value.as_ref().map(|v| v.as_ref()));

            return stmt.query_row(params_from_iter(params), |row| {
                Ok(T::try_from(row).map_err(Into::into))
            })?;
        }

        let (sql_str, params) = self.inner.bind(params);

        let mut stmt = conn.prepare(sql_str)?;

        stmt.query_row(params_from_iter(params), |row| {
            Ok(T::try_from(row).map_err(Into::into))
        })?
    }
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

impl OwnedPreparedStatement {
    #[inline]
    fn has_all_internal_params(&self) -> bool {
        self.inner.params.iter().all(|p| p.value.is_some())
    }

    /// Runs the prepared statement and returns the number of affected rows
    pub fn execute<'a>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = ParamBind<'a, SQLiteValue<'a>>>,
    ) -> Result<usize> {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "owned_prepared.execute");
        if self.has_all_internal_params() {
            let params = self.inner.params.iter().filter_map(|p| p.value.as_ref());

            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "owned_prepared.execute.db");
            return Ok(conn.execute(self.inner.sql(), params_from_iter(params))?);
        }

        let (sql_str, params) = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "owned_prepared.execute.bind");
            self.inner.bind(params)
        };

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "owned_prepared.execute.db");
        Ok(conn.execute(sql_str, params_from_iter(params))?)
    }

    /// Runs the prepared statement and returns all matching rows
    pub fn all<'a, T>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = ParamBind<'a, SQLiteValue<'a>>>,
    ) -> Result<Vec<T>>
    where
        T: for<'r> TryFrom<&'r Row<'r>>,
        for<'r> <T as TryFrom<&'r Row<'r>>>::Error: Into<DrizzleError>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "owned_prepared.all");
        if self.has_all_internal_params() {
            let mut stmt = conn.prepare(self.inner.sql())?;
            let params = self.inner.params.iter().filter_map(|p| p.value.as_ref());

            let mut rows = stmt.query_and_then(params_from_iter(params), |row| {
                T::try_from(row).map_err(Into::into)
            })?;

            let (lower, _) = rows.size_hint();
            let mut results = Vec::with_capacity(lower);
            for row in rows {
                results.push(row?);
            }

            return Ok(results);
        }

        let (sql_str, params) = self.inner.bind(params);

        let mut stmt = conn.prepare(sql_str)?;

        let mut rows = stmt.query_and_then(params_from_iter(params), |row| {
            T::try_from(row).map_err(Into::into)
        })?;

        let (lower, _) = rows.size_hint();
        let mut results = Vec::with_capacity(lower);
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    /// Runs the prepared statement and returns a single row
    pub fn get<'a, T>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = ParamBind<'a, SQLiteValue<'a>>>,
    ) -> Result<T>
    where
        T: for<'r> TryFrom<&'r Row<'r>>,
        for<'r> <T as TryFrom<&'r Row<'r>>>::Error: Into<DrizzleError>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("sqlite.rusqlite", "owned_prepared.get");
        if self.has_all_internal_params() {
            let mut stmt = conn.prepare(self.inner.sql())?;
            let params = self.inner.params.iter().filter_map(|p| p.value.as_ref());

            return stmt.query_row(params_from_iter(params), |row| {
                Ok(T::try_from(row).map_err(Into::into))
            })?;
        }

        let (sql_str, params) = self.inner.bind(params);

        let mut stmt = conn.prepare(sql_str)?;

        stmt.query_row(params_from_iter(params), |row| {
            Ok(T::try_from(row).map_err(Into::into))
        })?
    }
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
