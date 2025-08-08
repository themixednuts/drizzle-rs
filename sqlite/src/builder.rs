// Re-export common enums and traits from core
pub use drizzle_core::{
    OrderBy, SQL, ToSQL,
    traits::{IsInSchema, SQLSchema, SQLTable},
};

// Local imports
use crate::values::SQLiteValue;
use std::{fmt::Debug, marker::PhantomData};

// Import modules - these provide specific builder types
pub mod delete;
pub mod insert;
pub mod select;
pub mod update;

// Export state markers for easier use
pub use delete::{DeleteInitial, DeleteReturningSet, DeleteWhereSet};
pub use insert::{
    Conflict, InsertInitial, InsertOnConflictSet, InsertReturningSet, InsertValuesSet,
};
pub use select::{
    SelectFromSet, SelectGroupSet, SelectInitial, SelectJoinSet, SelectLimitSet, SelectOffsetSet,
    SelectOrderSet, SelectWhereSet,
};
pub use update::{UpdateInitial, UpdateReturningSet, UpdateSetClauseSet, UpdateWhereSet};

//------------------------------------------------------------------------------
// Common SQL Components
//------------------------------------------------------------------------------

/// Represents an ORDER BY clause in a query
#[derive(Debug, Clone)]
pub struct OrderByClause<'a> {
    /// The expression to order by
    pub expr: SQL<'a, SQLiteValue<'a>>,
    /// The direction to sort (ASC or DESC)
    pub direction: OrderBy,
}

impl<'a> OrderByClause<'a> {
    /// Creates a new ORDER BY clause
    pub const fn new(expr: SQL<'a, SQLiteValue<'a>>, direction: OrderBy) -> Self {
        Self { expr, direction }
    }
}

pub trait BuilderState {}
#[derive(Debug, Clone)]
pub struct BuilderInit;

impl BuilderState for BuilderInit {}
impl ExecutableState for BuilderInit {}

/// Main query builder for SQLite
///
/// The `S` type parameter represents the schema type, which is used
/// to ensure type safety when building queries.
#[derive(Debug, Clone, Default)]
pub struct QueryBuilder<'a, Schema = (), State = (), Table = ()> {
    pub(crate) sql: SQL<'a, SQLiteValue<'a>>,
    schema: PhantomData<Schema>,
    state: PhantomData<State>,
    table: PhantomData<Table>,
}

//------------------------------------------------------------------------------
// QueryBuilder Implementation
//------------------------------------------------------------------------------

impl<'a> QueryBuilder<'a> {
    /// Creates a new query builder for the given schema
    pub const fn new<S>() -> QueryBuilder<'a, S, BuilderInit> {
        QueryBuilder {
            sql: SQL::empty(),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

impl<'a, Schema, State> QueryBuilder<'a, Schema, State>
where
    State: BuilderState,
{
    pub fn select<T>(&self, columns: T) -> select::SelectBuilder<'a, Schema, select::SelectInitial>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let sql = crate::helpers::select(columns);
        select::SelectBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    pub fn insert<T>(&self) -> insert::InsertBuilder<'a, Schema, insert::InsertInitial, T>
    where
        T: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>>,
    {
        let sql = crate::helpers::insert::<T>();

        insert::InsertBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    pub fn update<T>(&self) -> update::UpdateBuilder<'a, Schema, update::UpdateInitial, T>
    where
        T: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>>,
    {
        let sql = crate::helpers::update::<T, SQLiteValue>();

        update::UpdateBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    pub fn delete<T>(&self) -> delete::DeleteBuilder<'a, Schema, delete::DeleteInitial, T>
    where
        T: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>>,
    {
        let sql = crate::helpers::delete::<T, SQLiteValue>();

        delete::DeleteBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}
impl<'a, Schema, State, Table> ToSQL<'a, SQLiteValue<'a>>
    for QueryBuilder<'a, Schema, State, Table>
{
    fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
        self.sql.clone()
    }
}

// Marker trait to indicate a query builder state is executable
pub trait ExecutableState {}

// Implementations for specific database drivers
//------------------------------------------------------------------------------

// RusQLite implementation
#[cfg(feature = "rusqlite")]
pub mod rusqlite_impl {
    use super::*;
    use ::rusqlite::{Connection, Row, params_from_iter};
    use drizzle_core::error::{DrizzleError, Result};

    impl<'a, Schema, State, Table> QueryBuilder<'a, Schema, State, Table>
    where
        State: ExecutableState,
    {
        // pub fn execute<T, P>(query: T, conn: &Connection) -> Result<usize>
        // where
        //     T: ToSQL<'a, SQLiteValue<'a>>,
        // {
        //     let q = query.to_sql();
        //     let sql = q.sql();
        //     let params = q.params();

        //     conn.execute(&sql, params_from_iter(params))
        //         .map_err(|e| DrizzleError::Other(e.to_string()))
        // }

        /// Runs the query and returns the number of affected rows
        pub fn execute(&self, conn: &Connection) -> Result<usize> {
            let sql = self.sql.sql();

            // Get parameters and handle potential errors from IntoParams
            let params = self.sql.params();

            conn.execute(&sql, params_from_iter(params))
                .map_err(|e| DrizzleError::Other(e.to_string()))
        }

        /// Runs the query and returns all matching rows
        pub fn all<T>(&self, conn: &Connection) -> Result<Vec<T>>
        where
            T: for<'r> TryFrom<&'r Row<'r>>,
            for<'r> <T as TryFrom<&'r Row<'r>>>::Error: Into<DrizzleError>,
        {
            let sql = self.sql.sql();

            // Get parameters and handle potential errors from IntoParams
            let params = self.sql.params();

            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| DrizzleError::Other(e.to_string()))?;

            let rows = stmt
                .query_map(params_from_iter(params), |row| {
                    Ok(T::try_from(row).map_err(Into::into))
                })
                .map_err(|e| DrizzleError::Other(e.to_string()))?;

            let mut results = Vec::new();
            for row in rows {
                results.push(row.map_err(|e| DrizzleError::Other(e.to_string()))??);
            }

            Ok(results)
        }

        pub fn get<T>(&self, conn: &Connection) -> Result<T>
        where
            T: for<'r> TryFrom<&'r Row<'r>>,
            for<'r> <T as TryFrom<&'r Row<'r>>>::Error: Into<DrizzleError>,
        {
            let sql = self.sql.sql();

            // Get parameters and handle potential errors from IntoParams
            let params = self.sql.params();

            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| DrizzleError::Other(e.to_string()))?;

            stmt.query_row(params_from_iter(params), |row| {
                Ok(T::try_from(row).map_err(Into::into))
            })
            .map_err(|e| DrizzleError::Other(e.to_string()))?
        }
    }
}

// LibSQL implementation can be added in a similar way when needed
#[cfg(feature = "libsql")]
pub mod libsql_impl {
    // Will implement similarly to rusqlite_impl when needed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder_new() {
        let qb = QueryBuilder::new::<()>();
        let sql = qb.to_sql();
        assert_eq!(sql.sql(), "");
        assert_eq!(sql.params().len(), 0);
    }

    #[test]
    fn test_builder_state_trait() {
        // Test that different states implement BuilderState
        fn assert_builder_state<T: BuilderState>() {}

        assert_builder_state::<BuilderInit>();
        // assert_builder_state::<SelectInitial>();
        // assert_builder_state::<InsertInitial>();
        // assert_builder_state::<UpdateInitial>();
        // assert_builder_state::<DeleteInitial>();
    }
}
