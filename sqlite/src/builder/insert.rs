// Contents of querybuilder/src/sqlite/insert.rs moved here
// querybuilder/src/sqlite/builder/insert.rs
use crate::values::SQLiteValue;
use drizzle_core::SQL;
use std::fmt::Debug;
use std::marker::PhantomData;

// Import the ExecutableState trait
use super::ExecutableState;

#[cfg(feature = "serde")]
use serde::de::DeserializeOwned;

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

/// Conflict resolution strategies for SQLite
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Do nothing on conflict (INSERT OR IGNORE)
    Ignore,
    /// Replace existing row on conflict (INSERT OR REPLACE)
    Replace,
    /// Abort transaction on conflict (INSERT OR ABORT)
    Abort,
    /// Fail with error on conflict (INSERT OR FAIL)
    Fail,
    /// Roll back transaction on conflict (INSERT OR ROLLBACK)
    Rollback,
}

// Mark states that can execute insert queries
impl ExecutableState for InsertValuesSet {}
impl ExecutableState for InsertReturningSet {}

//------------------------------------------------------------------------------
// InsertBuilder Definition
//------------------------------------------------------------------------------

/// Builds an INSERT query specifically for SQLite
pub type InsertBuilder<'a, Schema, State, Table> = super::QueryBuilder<'a, Schema, State, Table>;

//------------------------------------------------------------------------------
// Initial State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> InsertBuilder<'a, S, InsertInitial, T> {
    /// Sets values to insert and transitions to ValuesSet state
    pub fn values(
        self,
        values: Vec<Vec<SQL<'a, SQLiteValue<'a>>>>,
    ) -> InsertBuilder<'a, S, InsertValuesSet, T> {
        let values_sql = crate::helpers::values(values);
        InsertBuilder {
            sql: self.sql.append(values_sql),
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
        }
    }

    /// Sets conflict resolution strategy
    pub fn on_conflict(self, resolution: ConflictResolution) -> Self {
        // We'll handle conflict resolution by modifying the SQL directly
        let conflict_sql = match resolution {
            ConflictResolution::Ignore => "OR IGNORE",
            ConflictResolution::Replace => "OR REPLACE",
            ConflictResolution::Abort => "OR ABORT",
            ConflictResolution::Fail => "OR FAIL",
            ConflictResolution::Rollback => "OR ROLLBACK",
        };

        // Replace "INSERT INTO" with "INSERT [resolution] INTO"
        let current_sql = self.sql.clone();
        let modified_sql = SQL::raw(
            current_sql
                .sql()
                .replace("INSERT INTO", &format!("INSERT {} INTO", conflict_sql)),
        );

        InsertBuilder {
            sql: modified_sql,
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-VALUES Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> InsertBuilder<'a, S, InsertValuesSet, T> {
    /// Sets conflict resolution strategy
    pub fn on_conflict(self, resolution: ConflictResolution) -> Self {
        // We'll handle conflict resolution by modifying the SQL directly
        let conflict_sql = match resolution {
            ConflictResolution::Ignore => "OR IGNORE",
            ConflictResolution::Replace => "OR REPLACE",
            ConflictResolution::Abort => "OR ABORT",
            ConflictResolution::Fail => "OR FAIL",
            ConflictResolution::Rollback => "OR ROLLBACK",
        };

        // Replace "INSERT INTO" with "INSERT [resolution] INTO"
        let current_sql = self.sql.clone();
        let modified_sql = SQL::raw(
            current_sql
                .sql()
                .replace("INSERT INTO", &format!("INSERT {} INTO", conflict_sql)),
        );

        InsertBuilder {
            sql: modified_sql,
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
        }
    }

    /// Adds a RETURNING clause and transitions to ReturningSet state
    pub fn returning(
        self,
        columns: Vec<SQL<'a, SQLiteValue<'a>>>,
    ) -> InsertBuilder<'a, S, InsertReturningSet, T> {
        let returning_sql = crate::helpers::returning(columns);
        InsertBuilder {
            sql: self.sql.append(returning_sql),
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
        }
    }
}
