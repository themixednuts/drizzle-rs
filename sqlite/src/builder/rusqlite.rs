use crate::builder::{QueryBuilder, ExecutableState};
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