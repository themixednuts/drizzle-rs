use crate::builder::{QueryBuilder, ExecutableState};
use drizzle_core::error::{DrizzleError, Result};
use libsql::{Connection, Row};

impl<'a, Schema, State, Table> QueryBuilder<'a, Schema, State, Table>
where
    State: ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub async fn execute(&self, conn: &Connection) -> Result<u64> {
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
    pub async fn all<T>(&self, conn: &Connection) -> Result<Vec<T>>
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

    pub async fn get<T>(&self, conn: &Connection) -> Result<T>
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