use crate::builder::{ExecutableState, QueryBuilder};
use drizzle_core::ParamBind;
use drizzle_core::error::{DrizzleError, Result};
use turso::{Connection, IntoValue, Row};

impl<'a, Schema, State, Table> QueryBuilder<'a, Schema, State, Table>
where
    State: ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub async fn execute(&self, conn: &Connection) -> Result<u64> {
        let sql = self.sql.sql();
        let params: Vec<turso::Value> = self.sql.params().into_iter().map(|p| p.into()).collect();

        Ok(conn.execute(&sql, params).await?)
    }

    /// Runs the query and returns all matching rows
    pub async fn all<T>(&self, conn: &Connection) -> Result<Vec<T>>
    where
        T: for<'r> TryFrom<&'r Row>,
        for<'r> <T as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        let sql = &self.sql;
        let sql_str = sql.sql();
        let params: Vec<turso::Value> = sql.params().into_iter().map(|p| p.into()).collect();

        let mut rows = conn.query(&sql_str, params).await?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let converted = T::try_from(&row).map_err(Into::into)?;
            results.push(converted);
        }

        Ok(results)
    }

    pub async fn get<T>(&self, conn: &Connection) -> Result<T>
    where
        T: for<'r> TryFrom<&'r Row>,
        for<'r> <T as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        let sql = &self.sql;
        let sql_str = sql.sql();
        let params: Vec<turso::Value> = sql.params().into_iter().map(|p| p.into()).collect();

        let mut rows = conn.query(&sql_str, params).await?;

        if let Some(row) = rows.next().await? {
            T::try_from(&row).map_err(Into::into)
        } else {
            Err(DrizzleError::NotFound)
        }
    }
}

// Turso-specific execution methods for PreparedStatement
#[cfg(feature = "turso")]
impl<'a> crate::builder::prepared::PreparedStatement<'a> {
    /// Runs the prepared statement and returns the number of affected rows
    pub async fn execute(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = ParamBind<'a, crate::SQLiteValue<'a>>>,
    ) -> Result<u64> {
        // Convert to owned params and bind to pre-rendered SQL
        // Bind parameters to pre-rendered SQL
        let (sql_str, sql_params) = self.inner.clone().bind(params);

        // Convert to turso Values
        let turso_params: Vec<turso::Value> = sql_params.into_iter().map(|p| p.into()).collect();

        // Execute with connection
        let result = conn.execute(&sql_str, turso_params).await?;
        Ok(result)
    }

    /// Runs the prepared statement and returns all matching rows
    pub async fn all<T>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = ParamBind<'a, crate::SQLiteValue<'a>>>,
    ) -> Result<Vec<T>>
    where
        T: for<'r> TryFrom<&'r Row>,
        for<'r> <T as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        // Bind parameters to pre-rendered SQL
        let (sql_str, sql_params) = self.inner.clone().bind(params);

        // Convert to turso Values
        let turso_params: Vec<turso::Value> = sql_params.into_iter().map(|p| p.into()).collect();

        // Execute with connection
        let mut rows = conn.query(&sql_str, turso_params).await?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let converted = T::try_from(&row).map_err(Into::into)?;
            results.push(converted);
        }

        Ok(results)
    }

    /// Runs the prepared statement and returns a single row
    pub async fn get<T>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = ParamBind<'a, crate::SQLiteValue<'a>>>,
    ) -> Result<T>
    where
        T: for<'r> TryFrom<&'r Row>,
        for<'r> <T as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        // Convert to owned params and bind to pre-rendered SQL
        // Bind parameters to pre-rendered SQL
        let (sql_str, sql_params) = self.inner.clone().bind(params);

        // Convert to turso Values
        let turso_params: Vec<turso::Value> = sql_params.into_iter().map(|p| p.into()).collect();

        // Execute with connection
        let mut rows = conn.query(&sql_str, turso_params).await?;

        if let Some(row) = rows.next().await? {
            T::try_from(&row).map_err(Into::into)
        } else {
            Err(DrizzleError::NotFound)
        }
    }
}

// Turso-specific execution methods for OwnedPreparedStatement
#[cfg(feature = "turso")]
impl crate::builder::prepared::OwnedPreparedStatement {
    /// Runs the prepared statement and returns the number of affected rows
    pub async fn execute<'a>(
        &self,
        conn: &Connection,
        params: impl IntoIterator<Item = drizzle_core::ParamBind<'a, crate::SQLiteValue<'a>>>,
    ) -> Result<u64> {
        // Convert to owned params and bind to pre-rendered SQL
        use crate::values::OwnedSQLiteValue;
        let prepared_params = self.inner.params.iter().map(|v| &v.value).flat_map(|v| v);
        let runtime_params = params
            .into_iter()
            .map(|p| ParamBind::new(p.name, OwnedSQLiteValue::from(p.value)));

        let (sql_str, sql_params) = self.inner.clone().bind(runtime_params);

        let all_params = prepared_params.chain(sql_params.iter());

        // Convert to turso Values
        let turso_params: Vec<turso::Value> = all_params.into_iter().map(|p| p.into()).collect();

        // Execute with connection
        let result = conn.execute(&sql_str, turso_params).await?;
        Ok(result)
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
        use crate::values::OwnedSQLiteValue;
        let prepared_params = self.inner.params.iter().map(|v| &v.value).flat_map(|v| v);
        let runtime_params = params
            .into_iter()
            .map(|p| ParamBind::new(p.name, OwnedSQLiteValue::from(p.value)));

        let (sql_str, sql_params) = self.inner.clone().bind(runtime_params);

        let all_params = prepared_params.chain(sql_params.iter());

        // Convert to turso Values
        let turso_params: Vec<turso::Value> = all_params.into_iter().map(|p| p.into()).collect();

        // Execute with connection
        let mut rows = conn.query(&sql_str, turso_params).await?;

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
        use crate::values::OwnedSQLiteValue;
        let prepared_params = self.inner.params.iter().map(|v| &v.value).flat_map(|v| v);
        let runtime_params = params
            .into_iter()
            .map(|p| ParamBind::new(p.name, OwnedSQLiteValue::from(p.value)));

        let (sql_str, sql_params) = self.inner.clone().bind(runtime_params);

        let all_params = prepared_params.chain(sql_params.iter());

        // Convert to turso Values
        let turso_params: Vec<turso::Value> = all_params.into_iter().map(|p| p.into()).collect();

        // Execute with connection
        let mut rows = conn.query(&sql_str, turso_params).await?;

        if let Some(row) = rows.next().await? {
            T::try_from(&row).map_err(Into::into)
        } else {
            Err(DrizzleError::NotFound)
        }
    }
}
