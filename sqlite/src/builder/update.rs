use crate::values::SQLiteValue;
use drizzle_core::SQLTable;
use drizzle_core::{SQL, error::Result};
use std::fmt::Debug;
use std::marker::PhantomData;

// Import the ExecutableState trait
use super::ExecutableState;

#[cfg(feature = "serde")]
use serde::de::DeserializeOwned;

//------------------------------------------------------------------------------
// Type State Markers
//------------------------------------------------------------------------------

/// Marker for the initial state of UpdateBuilder
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateInitial;

/// Marker for the state after SET clause
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateSetClauseSet;

/// Marker for the state after WHERE clause
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateWhereSet;

/// Marker for the state after RETURNING clause
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateReturningSet;

// Mark states that can execute update queries
impl ExecutableState for UpdateSetClauseSet {}
impl ExecutableState for UpdateWhereSet {}
impl ExecutableState for UpdateReturningSet {}

//------------------------------------------------------------------------------
// UpdateBuilder Definition
//------------------------------------------------------------------------------

/// Builds an UPDATE query specifically for SQLite
pub type UpdateBuilder<'a, Schema, State, Table> = super::QueryBuilder<'a, Schema, State, Table>;

//------------------------------------------------------------------------------
// Initial State Implementation
//------------------------------------------------------------------------------

impl<'a, Schema, Table> UpdateBuilder<'a, Schema, UpdateInitial, Table>
where
    Table: SQLTable<'a, SQLiteValue<'a>>,
{
    /// Sets the values to update and transitions to the SetClauseSet state
    pub fn set(
        self,
        values: Table::Update,
    ) -> UpdateBuilder<'a, Schema, UpdateSetClauseSet, Table> {
        let sql = crate::helpers::set::<'a, Table, SQLiteValue<'a>>(values);
        UpdateBuilder {
            sql: self.sql.append(sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-SET Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> UpdateBuilder<'a, S, UpdateSetClauseSet, T> {
    /// Adds a WHERE condition and transitions to the WhereSet state
    pub fn r#where(
        self,
        condition: SQL<'a, SQLiteValue<'a>>,
    ) -> UpdateBuilder<'a, S, UpdateWhereSet, T> {
        let where_sql = crate::helpers::r#where(condition);
        UpdateBuilder {
            sql: self.sql.append(where_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Adds a RETURNING clause and transitions to the ReturningSet state
    pub fn returning(
        self,
        columns: Vec<SQL<'a, SQLiteValue<'a>>>,
    ) -> UpdateBuilder<'a, S, UpdateReturningSet, T> {
        let returning_sql = crate::helpers::returning(columns);
        UpdateBuilder {
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

impl<'a, S, T> UpdateBuilder<'a, S, UpdateWhereSet, T> {
    /// Adds a RETURNING clause after WHERE
    pub fn returning(
        self,
        columns: Vec<SQL<'a, SQLiteValue<'a>>>,
    ) -> UpdateBuilder<'a, S, UpdateReturningSet, T> {
        let returning_sql = crate::helpers::returning(columns);
        UpdateBuilder {
            sql: self.sql.append(returning_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}
