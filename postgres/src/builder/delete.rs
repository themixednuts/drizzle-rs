use crate::values::PostgresValue;
use core::fmt::Debug;
use core::marker::PhantomData;
use drizzle_core::ToSQL;

// Import the ExecutableState trait
use super::ExecutableState;

//------------------------------------------------------------------------------
// Type State Markers
//------------------------------------------------------------------------------

/// Marker for the initial state of DeleteBuilder
#[derive(Debug, Clone, Copy, Default)]
pub struct DeleteInitial;

/// Marker for the state after WHERE clause
#[derive(Debug, Clone, Copy, Default)]
pub struct DeleteWhereSet;

/// Marker for the state after RETURNING clause
#[derive(Debug, Clone, Copy, Default)]
pub struct DeleteReturningSet;

// Mark states that can execute delete queries
impl ExecutableState for DeleteInitial {}
impl ExecutableState for DeleteWhereSet {}
impl ExecutableState for DeleteReturningSet {}

//------------------------------------------------------------------------------
// DeleteBuilder Definition
//------------------------------------------------------------------------------

/// Builds a DELETE query specifically for PostgreSQL
pub type DeleteBuilder<'a, Schema, State, Table> = super::QueryBuilder<'a, Schema, State, Table>;

//------------------------------------------------------------------------------
// Initial State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> DeleteBuilder<'a, S, DeleteInitial, T> {
    /// Adds a WHERE condition to the query
    #[inline]
    pub fn r#where(
        self,
        condition: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> DeleteBuilder<'a, S, DeleteWhereSet, T> {
        let where_sql = crate::helpers::r#where(condition.to_sql());
        DeleteBuilder {
            sql: self.sql.append(where_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Adds a RETURNING clause to the query
    #[inline]
    pub fn returning(
        self,
        columns: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> DeleteBuilder<'a, S, DeleteReturningSet, T> {
        let returning_sql = crate::helpers::returning(columns);
        DeleteBuilder {
            sql: self.sql.append(returning_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-WHERE Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> DeleteBuilder<'a, S, DeleteWhereSet, T> {
    /// Adds a RETURNING clause after WHERE
    #[inline]
    pub fn returning(
        self,
        columns: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> DeleteBuilder<'a, S, DeleteReturningSet, T> {
        let returning_sql = crate::helpers::returning(columns);
        DeleteBuilder {
            sql: self.sql.append(returning_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use drizzle_core::{SQL, ToSQL};

    #[test]
    fn test_delete_builder_creation() {
        let builder = DeleteBuilder::<(), DeleteInitial, ()> {
            sql: SQL::raw("DELETE FROM test"),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        };

        assert_eq!(builder.to_sql().sql(), "DELETE FROM test");
    }
}
