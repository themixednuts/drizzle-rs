use crate::builder::{QueryBuilder, ExecutableState};
use drizzle_core::error::{DrizzleError, Result};
use drizzle_core::ParamBind;
use libsql::{Connection, Row};

impl<'a, Schema, State, Table> QueryBuilder<'a, Schema, State, Table>
where
    State: ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub async fn execute<'a>(&self, conn: &Connection) -> Result<u64> {
        let sql = self.sql.sql();
        let params: Vec<libsql::Value> = self
            .sql
            .params()
            .into_iter()
            .map(|p| p.into())
            .collect();

        conn.execute(&sql, params)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string()))
    }

    /// Runs the query and returns all matching rows
    pub async fn all<'a, T>(&self, conn: &Connection) -> Result<Vec<T>>
    where
        T: for<'r> TryFrom<&'r Row>,
        for<'r> <T as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        let sql = &self.sql;
        let sql_str = sql.sql();
        let params: Vec<libsql::Value> = sql
            .params()
            .into_iter()
            .map(|p| {
                // Convert SQLiteValue to libsql::Value
                match p {
                    crate::values::SQLiteValue::Null => libsql::Value::Null,
                    crate::values::SQLiteValue::Integer(i) => libsql::Value::Integer(*i),
                    crate::values::SQLiteValue::Real(f) => libsql::Value::Real(*f),
                    crate::values::SQLiteValue::Text(s) => libsql::Value::Text(s.to_string()),
                    crate::values::SQLiteValue::Blob(b) => libsql::Value::Blob(b.to_vec()),
                }
            })
            .collect();

        let mut rows = conn
            .query(&sql_str, params)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string()))?;

        let mut results = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string()))?
        {
            let converted = T::try_from(&row).map_err(Into::into)?;
            results.push(converted);
        }

        Ok(results)
    }

    pub async fn get<'a, T>(&self, conn: &Connection) -> Result<T>
    where
        T: for<'r> TryFrom<&'r Row>,
        for<'r> <T as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        let sql = &self.sql;
        let sql_str = sql.sql();
        let params: Vec<libsql::Value> = sql
            .params()
            .into_iter()
            .map(|p| {
                // Convert SQLiteValue to libsql::Value
                match p {
                    crate::values::SQLiteValue::Null => libsql::Value::Null,
                    crate::values::SQLiteValue::Integer(i) => libsql::Value::Integer(*i),
                    crate::values::SQLiteValue::Real(f) => libsql::Value::Real(*f),
                    crate::values::SQLiteValue::Text(s) => libsql::Value::Text(s.to_string()),
                    crate::values::SQLiteValue::Blob(b) => libsql::Value::Blob(b.to_vec()),
                }
            })
            .collect();

        let mut rows = conn
            .query(&sql_str, params)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string()))?;

        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| DrizzleError::Other(e.to_string()))?
        {
            T::try_from(&row).map_err(Into::into)
        } else {
            Err(DrizzleError::Other("No rows returned".to_string()))
        }
    }
}

// libsql-specific execution methods for PreparedStatement
#[cfg(feature = "libsql")]
impl<'a> crate::builder::prepared::PreparedStatement<'a> {
    /// Runs the prepared statement and returns the number of affected rows
    pub async fn execute<'a>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = ParamBind<'a, crate::SQLiteValue<'a>>>,
    ) -> Result<u64> {
        // Convert to owned params and bind to pre-rendered SQL
        let owned_params = params.into_iter().map(|p| p.into_owned()).collect::<Vec<_>>();
        let (sql_str, sql_params) = self.inner.sql.clone().bind(owned_params);

        // Convert to libsql Values
        let libsql_params: Vec<libsql::Value> = sql_params
            .into_iter()
            .map(|p| p.into())
            .collect();

        // Execute with connection
        conn.execute(&sql_str, libsql_params).await
    }

    /// Runs the prepared statement and returns all matching rows
    pub async fn all<'a, T>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = ParamBind<'a, crate::SQLiteValue<'a>>>,
    ) -> Result<Vec<T>>
    where
        T: for<'r> TryFrom<&'r Row>,
        for<'r> <T as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        // Convert to owned params and bind to pre-rendered SQL
        let owned_params = params.into_iter().map(|p| p.into_owned()).collect::<Vec<_>>();
        let (sql_str, sql_params) = self.inner.sql.clone().bind(owned_params);

        // Convert to libsql Values
        let libsql_params: Vec<libsql::Value> = sql_params
            .into_iter()
            .map(|p| p.into())
            .collect();

        // Execute with connection
        let mut rows = conn.query(&sql_str, libsql_params).await?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let converted = T::try_from(&row).map_err(Into::into)?;
            results.push(converted);
        }

        Ok(results)
    }

    /// Runs the prepared statement and returns a single row
    pub async fn get<'a, T>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = ParamBind<'a, crate::SQLiteValue<'a>>>,
    ) -> Result<T>
    where
        T: for<'r> TryFrom<&'r Row>,
        for<'r> <T as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        // Convert to owned params and bind to pre-rendered SQL
        let owned_params = params.into_iter().map(|p| p.into_owned()).collect::<Vec<_>>();
        let (sql_str, sql_params) = self.inner.sql.clone().bind(owned_params);

        // Convert to libsql Values
        let libsql_params: Vec<libsql::Value> = sql_params
            .into_iter()
            .map(|p| p.into())
            .collect();

        // Execute with connection
        let mut rows = conn.query(&sql_str, libsql_params).await?;

        if let Some(row) = rows.next().await? {
            T::try_from(&row).map_err(Into::into)
        } else {
            Err(DrizzleError::NotFound)
        }
    }
}

// libsql-specific execution methods for OwnedPreparedStatement
#[cfg(feature = "libsql")]
impl crate::builder::prepared::OwnedPreparedStatement {
    /// Runs the prepared statement and returns the number of affected rows
    pub async fn execute<'a>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = drizzle_core::ParamBind<'a, crate::SQLiteValue<'a>>>,
    ) -> Result<u64> {
        // Convert to owned params and bind to pre-rendered SQL
        let owned_params = params.into_iter().map(|p| p.into_owned()).collect::<Vec<_>>();
        let (sql_str, sql_params) = self.inner.sql.clone().bind(owned_params);

        // Convert to libsql Values
        let libsql_params: Vec<libsql::Value> = sql_params
            .into_iter()
            .map(|p| p.into())
            .collect();

        // Execute with connection
        conn.execute(&sql_str, libsql_params).await
    }

    /// Runs the prepared statement and returns all matching rows
    pub async fn all<'a, T>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = drizzle_core::ParamBind<'a, crate::SQLiteValue<'a>>>,
    ) -> Result<Vec<T>>
    where
        T: for<'r> TryFrom<&'r Row>,
        for<'r> <T as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        // Convert to owned params and bind to pre-rendered SQL
        let owned_params = params.into_iter().map(|p| p.into_owned()).collect::<Vec<_>>();
        let (sql_str, sql_params) = self.inner.sql.clone().bind(owned_params);

        // Convert to libsql Values
        let libsql_params: Vec<libsql::Value> = sql_params
            .into_iter()
            .map(|p| p.into())
            .collect();

        // Execute with connection
        let mut rows = conn.query(&sql_str, libsql_params).await?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let converted = T::try_from(&row).map_err(Into::into)?;
            results.push(converted);
        }

        Ok(results)
    }

    /// Runs the prepared statement and returns a single row
    pub async fn get<'a, T>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = drizzle_core::ParamBind<'a, crate::SQLiteValue<'a>>>,
    ) -> Result<T>
    where
        T: for<'r> TryFrom<&'r Row>,
        for<'r> <T as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        // Convert to owned params and bind to pre-rendered SQL
        let owned_params = params.into_iter().map(|p| p.into_owned()).collect::<Vec<_>>();
        let (sql_str, sql_params) = self.inner.sql.clone().bind(owned_params);

        // Convert to libsql Values
        let libsql_params: Vec<libsql::Value> = sql_params
            .into_iter()
            .map(|p| p.into())
            .collect();

        // Execute with connection
        let mut rows = conn.query(&sql_str, libsql_params).await?;

        if let Some(row) = rows.next().await? {
            T::try_from(&row).map_err(Into::into)
        } else {
            Err(DrizzleError::NotFound)
        }
    }
}