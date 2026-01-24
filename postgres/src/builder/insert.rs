use crate::traits::PostgresTable;
use crate::values::PostgresValue;
use drizzle_core::{SQL, ToSQL};
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

/// Conflict target specification for PostgreSQL ON CONFLICT clause
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum ConflictTarget<'a> {
    /// Target specific columns: ON CONFLICT (column1, column2, ...)
    Columns(SQL<'a, PostgresValue<'a>>),
    /// Target specific columns with WHERE clause for partial unique indexes
    /// ON CONFLICT (column1, column2, ...) WHERE condition
    ColumnsWhere {
        columns: SQL<'a, PostgresValue<'a>>,
        where_clause: SQL<'a, PostgresValue<'a>>,
    },
    /// Target a specific constraint by name: ON CONFLICT ON CONSTRAINT constraint_name
    Constraint(String),
}

/// Conflict resolution strategies for PostgreSQL
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Conflict<'a> {
    /// Do nothing on conflict - ON CONFLICT DO NOTHING or ON CONFLICT (target) DO NOTHING
    DoNothing {
        /// Optional target specification. If None, matches any conflict
        target: Option<ConflictTarget<'a>>,
    },
    /// Update on conflict - ON CONFLICT (target) DO UPDATE SET ...
    DoUpdate {
        /// Required target specification for DO UPDATE
        target: ConflictTarget<'a>,
        /// SET clause assignments (can use EXCLUDED.column to reference proposed values)
        set: SQL<'a, PostgresValue<'a>>,
        /// Optional WHERE clause for conditional updates
        /// Applied after SET: DO UPDATE SET ... WHERE condition
        where_clause: Option<SQL<'a, PostgresValue<'a>>>,
    },
}

impl<'a> Default for Conflict<'a> {
    fn default() -> Self {
        Self::DoNothing { target: None }
    }
}

impl<'a> Conflict<'a> {
    /// Create a DO NOTHING conflict resolution with specific columns
    pub fn do_nothing_on_columns<T>(columns: T) -> Self
    where
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        Conflict::DoNothing {
            target: Some(ConflictTarget::Columns(columns.to_sql())),
        }
    }

    /// Create a DO NOTHING conflict resolution with a constraint name
    pub fn do_nothing_on_constraint(constraint_name: String) -> Self {
        Conflict::DoNothing {
            target: Some(ConflictTarget::Constraint(constraint_name)),
        }
    }

    /// Create a DO UPDATE conflict resolution
    pub fn do_update(
        target: ConflictTarget<'a>,
        set: SQL<'a, PostgresValue<'a>>,
        where_clause: Option<SQL<'a, PostgresValue<'a>>>,
    ) -> Self {
        Conflict::DoUpdate {
            target,
            set,
            where_clause,
        }
    }
}

impl<'a> ConflictTarget<'a> {
    /// Create a column target
    pub fn columns<T>(columns: T) -> Self
    where
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        ConflictTarget::Columns(columns.to_sql())
    }

    /// Create a column target with WHERE clause for partial unique indexes
    pub fn columns_where<T>(columns: T, where_clause: SQL<'a, PostgresValue<'a>>) -> Self
    where
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        ConflictTarget::ColumnsWhere {
            columns: columns.to_sql(),
            where_clause,
        }
    }

    /// Create a constraint target
    pub fn constraint(constraint_name: String) -> Self {
        ConflictTarget::Constraint(constraint_name)
    }
}

// Mark states that can execute insert queries
impl ExecutableState for InsertValuesSet {}
impl ExecutableState for InsertReturningSet {}
impl ExecutableState for InsertOnConflictSet {}

//------------------------------------------------------------------------------
// InsertBuilder Definition
//------------------------------------------------------------------------------

/// Builds an INSERT query specifically for PostgreSQL
pub type InsertBuilder<'a, Schema, State, Table> = super::QueryBuilder<'a, Schema, State, Table>;

//------------------------------------------------------------------------------
// Initial State Implementation
//------------------------------------------------------------------------------

impl<'a, Schema, Table> InsertBuilder<'a, Schema, InsertInitial, Table>
where
    Table: PostgresTable<'a>,
{
    /// Sets values to insert and transitions to ValuesSet state
    #[inline]
    pub fn values<I, T>(self, values: I) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        I: IntoIterator<Item = Table::Insert<T>>,
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
    /// Adds conflict resolution clause following PostgreSQL ON CONFLICT syntax
    pub fn on_conflict(
        self,
        conflict: Conflict<'a>,
    ) -> InsertBuilder<'a, S, InsertOnConflictSet, T> {
        let conflict_sql = match conflict {
            Conflict::DoNothing { target } => {
                let mut sql = SQL::raw("ON CONFLICT");

                if let Some(target) = target {
                    sql = sql.append(Self::build_conflict_target(target));
                }

                sql.append(SQL::raw(" DO NOTHING"))
            }
            Conflict::DoUpdate {
                target,
                set,
                where_clause,
            } => {
                let mut sql = SQL::raw("ON CONFLICT")
                    .append(Self::build_conflict_target(target))
                    .append(SQL::raw(" DO UPDATE SET "))
                    .append(set);

                // Add optional WHERE clause for conditional updates
                if let Some(where_clause) = where_clause {
                    sql = sql.append(SQL::raw(" WHERE ")).append(where_clause);
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

    /// Helper method to build the conflict target portion of ON CONFLICT
    fn build_conflict_target(target: ConflictTarget<'a>) -> SQL<'a, PostgresValue<'a>> {
        match target {
            ConflictTarget::Columns(columns) => {
                SQL::raw(" (").append(columns).append(SQL::raw(")"))
            }
            ConflictTarget::ColumnsWhere {
                columns,
                where_clause,
            } => SQL::raw(" (")
                .append(columns)
                .append(SQL::raw(") WHERE "))
                .append(where_clause),
            ConflictTarget::Constraint(constraint_name) => {
                SQL::raw(" ON CONSTRAINT ").append(SQL::raw(constraint_name))
            }
        }
    }

    /// Shorthand for ON CONFLICT DO NOTHING (matches any conflict)
    pub fn on_conflict_do_nothing(self) -> InsertBuilder<'a, S, InsertOnConflictSet, T> {
        self.on_conflict(Conflict::default())
    }

    /// Shorthand for ON CONFLICT (columns...) DO NOTHING
    pub fn on_conflict_do_nothing_on<C>(
        self,
        columns: C,
    ) -> InsertBuilder<'a, S, InsertOnConflictSet, T>
    where
        C: ToSQL<'a, PostgresValue<'a>>,
    {
        self.on_conflict(Conflict::do_nothing_on_columns(columns))
    }

    /// Adds a RETURNING clause and transitions to ReturningSet state
    #[inline]
    pub fn returning(
        self,
        columns: impl ToSQL<'a, PostgresValue<'a>>,
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
        columns: impl ToSQL<'a, PostgresValue<'a>>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use drizzle_core::{SQL, ToSQL};

    #[test]
    fn test_insert_builder_creation() {
        let builder = InsertBuilder::<(), InsertInitial, ()> {
            sql: SQL::raw("INSERT INTO test"),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        };

        assert_eq!(builder.to_sql().sql(), "INSERT INTO test");
    }

    #[test]
    fn test_conflict_types() {
        let do_nothing = Conflict::DoNothing { target: None };
        let do_update = Conflict::DoUpdate {
            target: ConflictTarget::Columns(SQL::raw("id")),
            set: SQL::raw("name = EXCLUDED.name"),
            where_clause: None,
        };

        match do_nothing {
            Conflict::DoNothing { .. } => (),
            _ => panic!("Expected DoNothing"),
        }

        match do_update {
            Conflict::DoUpdate { .. } => (),
            _ => panic!("Expected DoUpdate"),
        }
    }
}
