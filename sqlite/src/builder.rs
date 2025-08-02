// Re-export common enums and traits from core
pub use drizzle_core::{
    Join, SQL, SortDirection, ToSQL,
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
pub use insert::{InsertInitial, InsertReturningSet, InsertValuesSet};
pub use select::{
    SelectFromSet, SelectGroupSet, SelectInitial, SelectJoinSet, SelectLimitSet, SelectOffsetSet,
    SelectOrderSet, SelectWhereSet,
};
pub use update::{UpdateInitial, UpdateReturningSet, UpdateSetClauseSet, UpdateWhereSet};

//------------------------------------------------------------------------------
// Common SQL Components
//------------------------------------------------------------------------------

/// Represents a JOIN clause in a query
#[derive(Debug, Clone)]
pub struct JoinClause<'a> {
    /// The type of join (INNER, LEFT, etc.)
    pub join_type: String,
    /// The table to join with
    pub table: String,
    /// The ON condition for the join
    pub condition: SQL<'a, SQLiteValue<'a>>,
}

/// Represents an ORDER BY clause in a query
#[derive(Debug, Clone)]
pub struct OrderByClause<'a> {
    /// The expression to order by
    pub expr: SQL<'a, SQLiteValue<'a>>,
    /// The direction to sort (ASC or DESC)
    pub direction: SortDirection,
}

pub trait BuilderState {}
#[derive(Debug, Clone)]
pub struct BuilderInit;

impl BuilderState for BuilderInit {}

/// Main query builder for SQLite
///
/// The `S` type parameter represents the schema type, which is used
/// to ensure type safety when building queries.
#[derive(Debug, Clone, Default)]
pub struct QueryBuilder<'a, Schema = (), State = (), Table = ()> {
    pub(crate) sql: SQL<'a, SQLiteValue<'a>>,
    _schema: PhantomData<Schema>,
    _state: PhantomData<State>,
    _table: PhantomData<Table>,
}

//------------------------------------------------------------------------------
// QueryBuilder Implementation
//------------------------------------------------------------------------------

impl<'a> QueryBuilder<'a> {
    /// Creates a new query builder for the given schema
    pub fn new<S>() -> QueryBuilder<'a, S, BuilderInit> {
        QueryBuilder {
            sql: SQL::default(),
            _schema: PhantomData::<S>,
            _state: PhantomData::<BuilderInit>,
            _table: PhantomData::<()>,
        }
    }
}

impl<'a, Schema, State> QueryBuilder<'a, Schema, State>
where
    State: BuilderState,
{
    pub fn select<const N: usize>(
        &self,
        columns: [impl ToSQL<'a, SQLiteValue<'a>>; N],
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial> {
        let sql = crate::helpers::select(columns);
        select::SelectBuilder {
            sql,
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData::<()>,
        }
    }

    pub fn insert<T>(&self) -> insert::InsertBuilder<'a, Schema, insert::InsertInitial, T>
    where
        T: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>>,
    {
        let table_name = T::Schema::NAME;
        let sql = crate::helpers::insert_into(SQL::raw(table_name));

        insert::InsertBuilder {
            sql,
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
        }
    }

    pub fn update<T>(&self) -> update::UpdateBuilder<'a, Schema, update::UpdateInitial, T>
    where
        T: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>>,
    {
        let table_name = T::Schema::NAME;
        let sql = crate::helpers::update(SQL::raw(table_name));

        update::UpdateBuilder {
            sql,
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
        }
    }

    pub fn delete<T>(&self) -> delete::DeleteBuilder<'a, Schema, delete::DeleteInitial, T>
    where
        T: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>>,
    {
        let table_name = T::Schema::NAME;
        let sql = crate::helpers::delete_from(SQL::raw(table_name));

        delete::DeleteBuilder {
            sql,
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
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

#[cfg(test)]
mod tests {

    #[test]
    fn test_select_builder() {}
}

// Marker trait to indicate a query builder state is executable
pub trait ExecutableState {}

// Implementations for specific database drivers
//------------------------------------------------------------------------------

// RusQLite implementation
#[cfg(feature = "rusqlite")]
pub mod rusqlite_impl {
    use super::*;
    use ::rusqlite::{self, Connection, Row, params_from_iter};
    use drizzle_core::error::{DrizzleError, Result};

    impl<'a, Schema, State, Table> QueryBuilder<'a, Schema, State, Table>
    where
        State: ExecutableState,
    {
        /// Executes the query and returns the number of affected rows
        pub fn execute(&self, conn: &Connection) -> Result<usize> {
            let sql = self.sql.sql();

            // Get parameters and handle potential errors from IntoParams
            let params = self.sql.params();

            // Convert SQLiteValue to rusqlite-compatible values
            let rusqlite_params: Vec<&dyn rusqlite::ToSql> = params
                .iter()
                .map(|val| val as &dyn rusqlite::ToSql)
                .collect();

            conn.execute(&sql, params_from_iter(rusqlite_params.into_iter()))
                .map_err(|e| DrizzleError::Other(e.to_string()))
        }

        /// Runs the query and returns all matching rows
        pub fn all<T>(&self, conn: &Connection) -> Result<Vec<T>>
        where
            T: for<'r> TryFrom<&'r Row<'r>>,
            for<'r> <T as TryFrom<&'r Row<'r>>>::Error: Into<rusqlite::Error>,
        {
            let sql = self.sql.sql();

            // Get parameters and handle potential errors from IntoParams
            let params = self.sql.params();

            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| DrizzleError::Other(e.to_string()))?;

            let rows = stmt
                .query_map(params_from_iter(params), |row| {
                    Ok(T::try_from(row).map_err(|e| e.into())?)
                })
                .map_err(|e| DrizzleError::Other(e.to_string()))?;

            let mut results = Vec::new();
            for row_result in rows {
                results.push(row_result.map_err(|e| DrizzleError::Other(e.to_string()))?);
            }

            Ok(results)
        }

        pub fn get<T>(&self, conn: &Connection) -> Result<T>
        where
            T: for<'r> TryFrom<&'r Row<'r>>,
            for<'r> <T as TryFrom<&'r Row<'r>>>::Error: Into<rusqlite::Error>,
        {
            let sql = self.sql.sql();

            // Get parameters and handle potential errors from IntoParams
            let params: Vec<SQLiteValue> = self.sql.params();

            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| DrizzleError::Other(e.to_string()))?;

            stmt.query_row(params_from_iter(params), |row| {
                Ok(T::try_from(row).map_err(|e| e.into())?)
            })
            .map_err(|e| DrizzleError::Other(e.to_string()))
        }
    }
}

// LibSQL implementation can be added in a similar way when needed
#[cfg(feature = "libsql")]
pub mod libsql_impl {
    // Will implement similarly to rusqlite_impl when needed
}
