use crate::{Connection, DriverError};
use drivers::{SQLiteValue, Transaction};
use querybuilder::sqlite::query_builder::SQLiteQueryBuilder;
use std::marker::PhantomData;

/// Represents an active database connection with optional schema information.
pub struct Drizzle<'conn, Conn, QB = ()> {
    pub(crate) connection: Conn,
    pub(crate) query_builder: Option<QB>,
    pub(crate) _lifetime: PhantomData<&'conn ()>,
}

// Base implementation
impl<'conn, Conn, QB> Drizzle<'conn, Conn, QB> {
    /// Creates a new Drizzle instance with just a connection
    pub fn new(connection: Conn) -> Drizzle<'conn, Conn, QB> {
        Self {
            connection,
            query_builder: None,
            _lifetime: PhantomData,
        }
    }

    /// Creates a new Drizzle instance with connection and schema
    pub fn with_schema(connection: Conn, query_builder: QB) -> Self {
        Self {
            connection,
            query_builder: Some(query_builder),
            _lifetime: PhantomData,
        }
    }
}

// Methods requiring Connection trait bounds
impl<'conn, Conn, QB> Drizzle<'conn, Conn, QB>
where
    // Assuming SQLiteValue works for all supported drivers
    Conn: Connection<Value = SQLiteValue<'conn>> + 'conn,
{
    /// Executes a raw SQL query string returning affected rows.
    /// This is the primary escape hatch.
    pub fn execute(&self, sql: &str, params: &[Conn::Value]) -> Result<usize, DriverError> {
        self.connection.run_statement(sql, params)
    }

    /// Prepares a SQL statement for potentially more efficient repeated execution.
    pub fn prepare(&self, sql: &str) -> Result<Conn::Prepared<'_>, DriverError> {
        self.connection.prepare(sql)
    }

    /// Executes a function within a database transaction.
    pub fn transaction<F, T, E>(&mut self, f: F) -> Result<T, E>
    where
        for<'tx> F: FnOnce(&mut Conn::Transaction<'tx>) -> Result<T, E>,
        E: From<DriverError>,
    {
        let mut tx = self.connection.begin_transaction().map_err(E::from)?;
        match f(&mut tx) {
            Ok(result) => {
                tx.commit().map_err(E::from)?;
                Ok(result)
            }
            Err(e) => {
                // Attempt rollback, but return the original error `e`
                let _ = tx.rollback(); // Consider logging rollback error?
                Err(e)
            }
        }
    }
}

// Schema-related functionality (SQLite specific for now)
impl<'conn, Conn, S> Drizzle<'conn, Conn, SQLiteQueryBuilder<'conn, S>>
where
    S: Clone,
    Conn: Connection<Value = SQLiteValue<'conn>> + 'conn, // Added connection bound here too
{
    /// Start building a query from a specific table defined in the schema.
    pub fn from<T>(&self) -> querybuilder::sqlite::query_builder::QueryBuilder<'conn, S>
    where
        T: querybuilder::core::schema_traits::IsInSchema<S>
            + querybuilder::core::traits::SQLSchema<
                'conn,
                querybuilder::sqlite::common::SQLiteTableType,
            >,
    {
        if let Some(qb) = &self.query_builder {
            // Get a query builder from the factory and set its table
            let mut query_builder = qb.query();
            query_builder.from::<T>();
            query_builder
        } else {
            // This branch should be unreachable if QB is SQLiteQueryBuilder
            // But kept for robustness or future generics
            panic!(
                "Cannot use schema-based queries with a Drizzle instance that doesn't have a schema"
            )
        }
    }
}
