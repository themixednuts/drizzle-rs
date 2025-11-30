use drizzle_core::prepared::owned::OwnedPreparedStatement;
use drizzle_postgres::PostgresValue;

/// A prepared statement that can be executed multiple times with different parameters.
///
/// This is a wrapper around an owned prepared statement that can be used with the postgres driver.
#[derive(Debug)]
pub struct PreparedStatement<'a> {
    pub(super) inner: OwnedPreparedStatement<PostgresValue<'a>>,
}

impl<'a> PreparedStatement<'a> {
    /// Gets the SQL query string by reconstructing it from text segments
    pub fn sql(&self) -> String {
        use drizzle_core::ToSQL;
        self.inner.to_sql().sql()
    }

    /// Gets the number of parameters in the query
    pub fn param_count(&self) -> usize {
        self.inner.params.len()
    }
}
