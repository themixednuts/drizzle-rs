use crate::common::PostgresSchemaType;
use crate::values::PostgresValue;
use drizzle_core::{SQLTable, ToSQL};
use std::fmt::Debug;
use std::marker::PhantomData;

// Import the ExecutableState trait
use super::ExecutableState;

//------------------------------------------------------------------------------
// Type State Markers
//------------------------------------------------------------------------------

/// Marker for the initial state of UpdateBuilder
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateInitial;

/// Marker for the state after SET clause
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateSetClauseSet;

/// Marker for the state after FROM clause
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateFromSet;

/// Marker for the state after WHERE clause
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateWhereSet;

/// Marker for the state after RETURNING clause
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateReturningSet;

// Mark states that can execute update queries
impl ExecutableState for UpdateSetClauseSet {}
impl ExecutableState for UpdateFromSet {}
impl ExecutableState for UpdateWhereSet {}
impl ExecutableState for UpdateReturningSet {}

//------------------------------------------------------------------------------
// UpdateBuilder Definition
//------------------------------------------------------------------------------

/// Builds an UPDATE query specifically for PostgreSQL
pub type UpdateBuilder<'a, Schema, State, Table> = super::QueryBuilder<'a, Schema, State, Table>;

//------------------------------------------------------------------------------
// Initial State Implementation
//------------------------------------------------------------------------------

impl<'a, Schema, Table> UpdateBuilder<'a, Schema, UpdateInitial, Table>
where
    Table: SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>,
{
    /// Sets the values to update and transitions to the SetClauseSet state
    #[inline]
    pub fn set(
        self,
        values: Table::Update,
    ) -> UpdateBuilder<'a, Schema, UpdateSetClauseSet, Table> {
        let sql = crate::helpers::set::<'a, Table, PostgresSchemaType, PostgresValue<'a>>(values);
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
    /// Adds a FROM clause and transitions to the FromSet state
    #[inline]
    pub fn from(
        self,
        source: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> UpdateBuilder<'a, S, UpdateFromSet, T> {
        let from_sql = crate::helpers::from(source);
        UpdateBuilder {
            sql: self.sql.append(from_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Adds a WHERE condition and transitions to the WhereSet state
    #[inline]
    pub fn r#where(
        self,
        condition: impl ToSQL<'a, PostgresValue<'a>>,
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
    #[inline]
    pub fn returning(
        self,
        columns: impl ToSQL<'a, PostgresValue<'a>>,
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
// Post-FROM Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> UpdateBuilder<'a, S, UpdateFromSet, T> {
    /// Adds a WHERE condition after FROM
    #[inline]
    pub fn r#where(
        self,
        condition: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> UpdateBuilder<'a, S, UpdateWhereSet, T> {
        let where_sql = crate::helpers::r#where(condition);
        UpdateBuilder {
            sql: self.sql.append(where_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Adds a RETURNING clause after FROM
    #[inline]
    pub fn returning(
        self,
        columns: impl ToSQL<'a, PostgresValue<'a>>,
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
    #[inline]
    pub fn returning(
        self,
        columns: impl ToSQL<'a, PostgresValue<'a>>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use drizzle_core::{SQL, ToSQL};

    #[test]
    fn test_update_builder_creation() {
        let builder = UpdateBuilder::<(), UpdateInitial, ()> {
            sql: SQL::raw("UPDATE test"),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        };

        assert_eq!(builder.to_sql().sql(), "UPDATE test");
    }
}
