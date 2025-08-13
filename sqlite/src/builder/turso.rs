use crate::builder::{QueryBuilder, ExecutableState};
use drizzle_core::error::{DrizzleError, Result};
use turso::{Connection, IntoValue, Row};

impl<'a, Schema, State, Table> QueryBuilder<'a, Schema, State, Table>
where
    State: ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub async fn execute(&self, conn: &Connection) -> Result<u64> {
        let sql = self.sql.sql();
        let params: Vec<turso::Value> = self
            .sql
            .params()
            .into_iter()
            .map(|p| {
                p.into_value()
                    .map_err(|e| DrizzleError::Other(e.to_string()))
            })
            .collect::<Result<Vec<_>>>()?;

        let result = conn
            .execute(&sql, params)
            .await
            .map_err(|e| DrizzleError::Other(e.to_string()))?;

        Ok(result)
    }

    /// Runs the query and returns all matching rows
    pub async fn all<T>(&self, conn: &Connection) -> Result<Vec<T>>
    where
        T: for<'r> TryFrom<&'r Row>,
        for<'r> <T as TryFrom<&'r Row>>::Error: Into<DrizzleError>,
    {
        let sql = &self.sql;
        let sql_str = sql.sql();
        let params: Vec<turso::Value> = sql
            .params()
            .into_iter()
            .map(|p| {
                p.into_value()
                    .map_err(|e| DrizzleError::Other(e.to_string()))
            })
            .collect::<Result<Vec<_>>>()?;

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
        let params: Vec<turso::Value> = sql
            .params()
            .into_iter()
            .map(|p| {
                p.into_value()
                    .map_err(|e| DrizzleError::Other(e.to_string()))
            })
            .collect::<Result<Vec<_>>>()?;

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