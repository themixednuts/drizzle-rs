use crate::values::SQLiteValue;
use drizzle_core::{SQL, SQLModel, SQLTable, ToSQL};
use std::fmt::Debug;
use std::marker::PhantomData;

// Import the ExecutableState trait
use super::ExecutableState;

//------------------------------------------------------------------------------
// Type State Markers
//------------------------------------------------------------------------------

/// Marker for the initial state of InsertBuilder.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertInitial;

/// Marker for the state after VALUES are set.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertValuesSet;

/// Marker for the state after RETURNING clause is added.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertReturningSet;

/// Marker for the state after ON CONFLICT is set.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertOnConflictSet;

// Const constructors for insert marker types
impl InsertInitial {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}
impl InsertValuesSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}
impl InsertReturningSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}
impl InsertOnConflictSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}

/// Conflict resolution strategies
#[derive(Debug, Clone)]
pub enum Conflict<
    'a,
    T: IntoIterator<Item: ToSQL<'a, SQLiteValue<'a>>> = Vec<SQL<'a, SQLiteValue<'a>>>,
> {
    /// Do nothing on conflict - ON CONFLICT DO NOTHING
    Ignore {
        /// Optional target columns to specify which constraint triggers the conflict
        target: Option<T>,
    },
    /// Update on conflict - ON CONFLICT DO UPDATE
    Update {
        /// Target columns that trigger the conflict
        target: T,
        /// SET clause for what to update
        set: SQL<'a, SQLiteValue<'a>>,
        /// Optional WHERE clause for the conflict target (partial indexes)
        /// This goes after the target: ON CONFLICT (col) WHERE condition
        target_where: Option<SQL<'a, SQLiteValue<'a>>>,
        /// Optional WHERE clause for the update (conditional updates)
        /// This goes after the SET: DO UPDATE SET col = val WHERE condition
        set_where: Option<SQL<'a, SQLiteValue<'a>>>,
    },
}

impl<'a> Default for Conflict<'a> {
    fn default() -> Self {
        Self::Ignore { target: None }
    }
}

// Mark states that can execute insert queries
impl ExecutableState for InsertValuesSet {}
impl ExecutableState for InsertReturningSet {}
impl ExecutableState for InsertOnConflictSet {}

//------------------------------------------------------------------------------
// InsertBuilder Definition
//------------------------------------------------------------------------------

/// Builds an INSERT query specifically for SQLite
pub type InsertBuilder<'a, Schema, State, Table> = super::QueryBuilder<'a, Schema, State, Table>;

//------------------------------------------------------------------------------
// Initial State Implementation
//------------------------------------------------------------------------------

impl<'a, Schema, Table> InsertBuilder<'a, Schema, InsertInitial, Table>
where
    Table: SQLTable<'a, SQLiteValue<'a>>,
{
    /// Sets values to insert and transitions to ValuesSet state
    #[inline]
    pub fn values<I, T>(self, values: I) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        I: IntoIterator<Item = Table::Insert<T>>,
        Table::Insert<T>: SQLModel<'a, SQLiteValue<'a>>,
    {
        let sql = crate::helpers::values::<'a, Table, T>(values);
        InsertBuilder {
            sql: self.sql.append(sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-VALUES Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> InsertBuilder<'a, S, InsertValuesSet, T> {
    /// Adds conflict resolution clause
    pub fn on_conflict<TI>(
        self,
        conflict: Conflict<'a, TI>,
    ) -> InsertBuilder<'a, S, InsertOnConflictSet, T>
    where
        TI: IntoIterator,
        TI::Item: ToSQL<'a, SQLiteValue<'a>>,
    {
        let conflict_sql = match conflict {
            Conflict::Ignore { target } => {
                if let Some(target_iter) = target {
                    let cols = SQL::join(target_iter.into_iter().map(|item| item.to_sql()), ", ");
                    SQL::raw("ON CONFLICT (")
                        .append(cols)
                        .append_raw(") DO NOTHING")
                } else {
                    SQL::raw("ON CONFLICT DO NOTHING")
                }
            }
            Conflict::Update {
                target,
                set,
                target_where,
                set_where,
            } => {
                let target_cols = SQL::join(target.into_iter().map(|item| item.to_sql()), ", ");
                let mut sql = SQL::raw("ON CONFLICT (")
                    .append(target_cols)
                    .append_raw(")");

                // Add target WHERE clause (for partial indexes)
                if let Some(target_where) = target_where {
                    sql = sql.append_raw(" WHERE ").append(target_where);
                }

                sql = sql.append_raw(" DO UPDATE SET ").append(set);

                // Add set WHERE clause (for conditional updates)
                if let Some(set_where) = set_where {
                    sql = sql.append_raw(" WHERE ").append(set_where);
                }

                sql
            }
        };

        InsertBuilder {
            sql: self.sql.append(conflict_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Adds a RETURNING clause and transitions to ReturningSet state
    #[inline]
    pub fn returning(
        self,
        columns: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> InsertBuilder<'a, S, InsertReturningSet, T> {
        let returning_sql = crate::helpers::returning(columns);
        InsertBuilder {
            sql: self.sql.append(returning_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-ON CONFLICT Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> InsertBuilder<'a, S, InsertOnConflictSet, T> {
    /// Adds a RETURNING clause after ON CONFLICT
    #[inline]
    pub fn returning(
        self,
        columns: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> InsertBuilder<'a, S, InsertReturningSet, T> {
        let returning_sql = crate::helpers::returning(columns);
        InsertBuilder {
            sql: self.sql.append(returning_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}
