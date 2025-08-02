use crate::helpers;
use crate::values::SQLiteValue;
use drizzle_core::traits::{IsInSchema, SQLSchema, SQLTable};
use drizzle_core::{Join, SQL, SortDirection};
use std::fmt::Debug;
use std::marker::PhantomData;

// Import the ExecutableState trait
use super::ExecutableState;

#[cfg(feature = "serde")]
use serde::de::DeserializeOwned;

//------------------------------------------------------------------------------
// Type State Markers
//------------------------------------------------------------------------------

/// Marker for the initial state of SelectBuilder.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectInitial;

/// Marker for the state after FROM clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectFromSet;

/// Marker for the state after JOIN clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectJoinSet;

/// Marker for the state after WHERE clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectWhereSet;

/// Marker for the state after GROUP BY clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectGroupSet;

/// Marker for the state after ORDER BY clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectOrderSet;

/// Marker for the state after LIMIT clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectLimitSet;

/// Marker for the state after OFFSET clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectOffsetSet;

// Mark states that can execute queries as implementing the ExecutableState trait
impl ExecutableState for SelectFromSet {}
impl ExecutableState for SelectWhereSet {}
impl ExecutableState for SelectLimitSet {}
impl ExecutableState for SelectOffsetSet {}
impl ExecutableState for SelectOrderSet {}
impl ExecutableState for SelectGroupSet {}
impl ExecutableState for SelectJoinSet {}

//------------------------------------------------------------------------------
// SelectBuilder Definition
//------------------------------------------------------------------------------

/// Builds a SELECT query specifically for SQLite
pub type SelectBuilder<'a, Schema, State, Table = ()> =
    super::QueryBuilder<'a, Schema, State, Table>;

//------------------------------------------------------------------------------
// Initial State Implementation
//------------------------------------------------------------------------------

impl<'a, S> SelectBuilder<'a, S, SelectInitial> {
    /// Specifies the table to select FROM and transitions state
    pub fn from<T>(self) -> SelectBuilder<'a, S, SelectFromSet, T>
    where
        T: SQLTable<'a, SQLiteValue<'a>> + IsInSchema<S>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::from(SQL::raw(T::Schema::NAME))),
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-FROM State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> SelectBuilder<'a, S, SelectFromSet, T>
where
    T: SQLTable<'a, SQLiteValue<'a>>,
{
    /// Adds a JOIN clause to the query
    pub fn join<U: IsInSchema<S> + SQLTable<'a, SQLiteValue<'a>>>(
        self,
        join_type: Join,
        on_condition: SQL<'a, SQLiteValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectJoinSet, T> {
        SelectBuilder {
            sql: self
                .sql
                .append(helpers::join(join_type, U::Schema::NAME, on_condition)),
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
        }
    }

    /// Adds a WHERE condition to the query
    pub fn r#where(
        self,
        condition: SQL<'a, SQLiteValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectWhereSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::where_clause(condition)),
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
        }
    }

    /// Adds a GROUP BY clause to the query
    pub fn group_by(
        self,
        expressions: Vec<SQL<'a, SQLiteValue<'a>>>,
    ) -> SelectBuilder<'a, S, SelectGroupSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::group_by(expressions)),
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
        }
    }

    /// Limits the number of rows returned
    pub fn limit(self, limit: usize) -> SelectBuilder<'a, S, SelectLimitSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::limit(limit)),
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
        }
    }

    /// Sets the offset for the query results
    pub fn offset(self, offset: usize) -> SelectBuilder<'a, S, SelectOffsetSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::offset(offset)),
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
        }
    }

    /// Sorts the query results
    pub fn order_by(
        self,
        expressions: Vec<(SQL<'a, SQLiteValue<'a>>, SortDirection)>,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::order_by(expressions)),
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-JOIN State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> SelectBuilder<'a, S, SelectJoinSet, T> {
    /// Adds a WHERE condition after a JOIN
    pub fn r#where(
        self,
        condition: SQL<'a, SQLiteValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectWhereSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::where_clause(condition)),
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-WHERE State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> SelectBuilder<'a, S, SelectWhereSet, T> {
    /// Adds a GROUP BY clause after a WHERE
    pub fn group_by(
        self,
        expressions: Vec<SQL<'a, SQLiteValue<'a>>>,
    ) -> SelectBuilder<'a, S, SelectGroupSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::group_by(expressions)),
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
        }
    }

    /// Adds an ORDER BY clause after a WHERE
    pub fn order_by(
        self,
        expressions: Vec<(SQL<'a, SQLiteValue<'a>>, SortDirection)>,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::order_by(expressions)),
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
        }
    }

    /// Adds a LIMIT clause after a WHERE
    pub fn limit(self, limit: usize) -> SelectBuilder<'a, S, SelectLimitSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::limit(limit)),
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-GROUP BY State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> SelectBuilder<'a, S, SelectGroupSet, T> {
    /// Adds a HAVING clause after GROUP BY
    pub fn having(
        self,
        condition: SQL<'a, SQLiteValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectGroupSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::having(condition)),
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
        }
    }

    /// Adds an ORDER BY clause after GROUP BY
    pub fn order_by(
        self,
        expressions: Vec<(SQL<'a, SQLiteValue<'a>>, SortDirection)>,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::order_by(expressions)),
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-ORDER BY State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> SelectBuilder<'a, S, SelectOrderSet, T> {
    /// Adds a LIMIT clause after ORDER BY
    pub fn limit(self, limit: usize) -> SelectBuilder<'a, S, SelectLimitSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::limit(limit)),
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-LIMIT State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> SelectBuilder<'a, S, SelectLimitSet, T> {
    /// Adds an OFFSET clause after LIMIT
    pub fn offset(self, offset: usize) -> SelectBuilder<'a, S, SelectOffsetSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::offset(offset)),
            _schema: PhantomData,
            _state: PhantomData,
            _table: PhantomData,
        }
    }
}
