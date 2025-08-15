#[cfg(feature = "rusqlite")]
use crate::SQLiteValue;
use crate::builder::{ExecutableState, QueryBuilder};
use drizzle_core::ParamBind;
use drizzle_core::error::{DrizzleError, Result};
use rusqlite::{Connection, Row, params_from_iter};

impl<'a, Schema, State, Table> QueryBuilder<'a, Schema, State, Table>
where
    State: ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub fn execute(&self, conn: &Connection) -> Result<usize> {
        let sql = self.sql.sql();

        // Get parameters and handle potential errors from IntoParams
        let params = self.sql.params();

        Ok(conn.execute(&sql, params_from_iter(params))?)
    }

    /// Runs the query and returns all matching rows
    pub fn all<T>(&self, conn: &Connection) -> Result<Vec<T>>
    where
        T: for<'r> TryFrom<&'r Row<'r>>,
        for<'r> <T as TryFrom<&'r Row<'r>>>::Error: Into<DrizzleError>,
    {
        let sql = &self.sql;
        let sql_str = sql.sql();

        let params = sql.params();

        let mut stmt = conn
            .prepare(&sql_str)
            .map_err(|e| DrizzleError::Other(e.to_string()))?;

        let rows = stmt
            .query_map(params_from_iter(params), |row| {
                Ok(T::try_from(row).map_err(Into::into))
            })
            .map_err(|e| DrizzleError::Other(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row??);
        }

        Ok(results)
    }

    pub fn get<T>(&self, conn: &Connection) -> Result<T>
    where
        T: for<'r> TryFrom<&'r Row<'r>>,
        for<'r> <T as TryFrom<&'r Row<'r>>>::Error: Into<DrizzleError>,
    {
        let sql = &self.sql;
        let sql_str = sql.sql();

        // Get parameters and handle potential errors from IntoParams
        let params = sql.params();

        let mut stmt = conn.prepare(&sql_str)?;

        stmt.query_row(params_from_iter(params), |row| {
            Ok(T::try_from(row).map_err(Into::into))
        })?
    }
}

// Rusqlite-specific execution methods for PreparedStatement
#[cfg(feature = "rusqlite")]
impl<'a> crate::builder::prepared::PreparedStatement<'a> {
    /// Runs the prepared statement and returns the number of affected rows
    pub fn execute(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = ParamBind<'a, crate::SQLiteValue<'a>>>,
    ) -> Result<usize> {
        // Bind parameters to pre-rendered SQL
        let (sql_str, sql_params) = self.inner.clone().bind(params);

        // Execute with connection
        Ok(conn.execute(&sql_str, params_from_iter(sql_params))?)
    }

    /// Runs the prepared statement and returns all matching rows
    pub fn all<T>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = ParamBind<'a, crate::SQLiteValue<'a>>>,
    ) -> Result<Vec<T>>
    where
        T: for<'r> TryFrom<&'r Row<'r>>,
        for<'r> <T as TryFrom<&'r Row<'r>>>::Error: Into<DrizzleError>,
    {
        // Bind parameters to pre-rendered SQL
        let (sql_str, sql_params) = self.inner.clone().bind(params);

        // Execute with connection
        let mut stmt = conn.prepare(&sql_str)?;

        let rows = stmt.query_map(params_from_iter(sql_params), |row| {
            Ok(T::try_from(row).map_err(Into::into))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row??);
        }

        Ok(results)
    }

    /// Runs the prepared statement and returns a single row
    pub fn get<T>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = ParamBind<'a, crate::SQLiteValue<'a>>>,
    ) -> Result<T>
    where
        T: for<'r> TryFrom<&'r Row<'r>>,
        for<'r> <T as TryFrom<&'r Row<'r>>>::Error: Into<DrizzleError>,
    {
        // Bind parameters to pre-rendered SQL
        let (sql_str, sql_params) = self.inner.clone().bind(params);

        // Execute with connection
        let mut stmt = conn.prepare(&sql_str)?;

        stmt.query_row(params_from_iter(sql_params), |row| {
            Ok(T::try_from(row).map_err(Into::into))
        })?
    }
}

// Rusqlite-specific execution methods for OwnedPreparedStatement
#[cfg(feature = "rusqlite")]
impl crate::builder::prepared::OwnedPreparedStatement {
    /// Runs the prepared statement and returns the number of affected rows
    pub fn execute<'a>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = drizzle_core::ParamBind<'a, crate::SQLiteValue<'a>>>,
    ) -> Result<usize> {
        // Convert to owned params and bind to pre-rendered SQL
        use crate::values::OwnedSQLiteValue;
        let prepared_params = self.inner.params.iter().map(|v| &v.value).flat_map(|v| v);
        let runtime_params = params
            .into_iter()
            .map(|p| ParamBind::new(p.name, OwnedSQLiteValue::from(p.value)));

        let (sql_str, sql_params) = self.inner.clone().bind(runtime_params);

        let all_params = prepared_params.chain(sql_params.iter());
        // Execute with connection
        Ok(conn.execute(&sql_str, params_from_iter(all_params))?)
    }

    /// Runs the prepared statement and returns all matching rows
    pub fn all<'a, T>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = drizzle_core::ParamBind<'a, crate::SQLiteValue<'a>>>,
    ) -> Result<Vec<T>>
    where
        T: for<'r> TryFrom<&'r Row<'r>>,
        for<'r> <T as TryFrom<&'r Row<'r>>>::Error: Into<DrizzleError>,
    {
        // Convert to owned params and bind to pre-rendered SQL
        use crate::values::OwnedSQLiteValue;
        let prepared_params = self.inner.params.iter().map(|v| &v.value).flat_map(|v| v);
        let runtime_params = params
            .into_iter()
            .map(|p| ParamBind::new(p.name, OwnedSQLiteValue::from(p.value)));

        let (sql_str, sql_params) = self.inner.clone().bind(runtime_params);

        let all_params = prepared_params.chain(sql_params.iter());

        // Execute with connection
        let mut stmt = conn.prepare(&sql_str)?;

        let rows = stmt.query_and_then(params_from_iter(all_params), |row| {
            T::try_from(row).map_err(Into::into)
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    /// Runs the prepared statement and returns a single row
    pub fn get<'a, T>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = drizzle_core::ParamBind<'a, crate::SQLiteValue<'a>>>,
    ) -> Result<T>
    where
        T: for<'r> TryFrom<&'r Row<'r>>,
        for<'r> <T as TryFrom<&'r Row<'r>>>::Error: Into<DrizzleError>,
    {
        // Convert to owned params and bind to pre-rendered SQL

        use crate::values::OwnedSQLiteValue;
        let prepared_params = self.inner.params.iter().map(|v| &v.value).flat_map(|v| v);
        let runtime_params = params
            .into_iter()
            .map(|p| ParamBind::new(p.name, OwnedSQLiteValue::from(p.value)));

        let (sql_str, sql_params) = self.inner.clone().bind(runtime_params);

        let all_params = prepared_params.chain(sql_params.iter());

        // Execute with connection
        let mut stmt = conn.prepare(&sql_str)?;

        stmt.query_row(params_from_iter(all_params), |row| {
            Ok(T::try_from(row).map_err(Into::into))
        })?
    }
}
