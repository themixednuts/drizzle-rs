use crate::values::SQLiteValue;
use drizzle_core::{SQL, ToSQL};
use std::fmt::Debug;
use std::marker::PhantomData;

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

/// Builds a DELETE query specifically for SQLite
pub type DeleteBuilder<'a, Schema, State, Table> = super::QueryBuilder<'a, Schema, State, Table>;

//------------------------------------------------------------------------------
// Initial State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> DeleteBuilder<'a, S, DeleteInitial, T> {
    /// Adds a WHERE condition to the query
    pub fn r#where(
        self,
        condition: SQL<'a, SQLiteValue<'a>>,
    ) -> DeleteBuilder<'a, S, DeleteWhereSet, T> {
        let where_sql = crate::helpers::r#where(condition);
        DeleteBuilder {
            sql: self.sql.append(where_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Adds a RETURNING clause to the query
    pub fn returning(
        self,
        columns: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> DeleteBuilder<'a, S, DeleteReturningSet, T> {
        let returning_sql = crate::helpers::returning(columns);
        DeleteBuilder {
            sql: self.sql.append(returning_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-WHERE Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> DeleteBuilder<'a, S, DeleteWhereSet, T> {
    /// Adds a RETURNING clause after WHERE
    pub fn returning(
        self,
        columns: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> DeleteBuilder<'a, S, DeleteReturningSet, T> {
        let returning_sql = crate::helpers::returning(columns);
        DeleteBuilder {
            sql: self.sql.append(returning_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}
