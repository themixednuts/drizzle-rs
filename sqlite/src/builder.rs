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
pub mod prepared;
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

#[derive(Debug, Clone)]
pub struct CTEInit;

impl BuilderState for BuilderInit {}
impl ExecutableState for BuilderInit {}

impl ExecutableState for CTEInit {}

/// Main query builder for SQLite
///
/// The `S` type parameter represents the schema type, which is used
/// to ensure type safety when building queries.
#[derive(Debug, Clone, Default)]
pub struct QueryBuilder<'a, Schema = (), State = (), Table = ()> {
    pub sql: SQL<'a, SQLiteValue<'a>>,
    schema: PhantomData<Schema>,
    state: PhantomData<State>,
    table: PhantomData<Table>,
}

//------------------------------------------------------------------------------
// QueryBuilder Implementation
//------------------------------------------------------------------------------

impl<'a, Schema, State, Table> ToSQL<'a, SQLiteValue<'a>>
    for QueryBuilder<'a, Schema, State, Table>
{
    fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
        self.sql.clone()
    }
}

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
}

impl<'a, Schema> QueryBuilder<'a, Schema, CTEInit> {
    pub fn select<T>(&self, columns: T) -> select::SelectBuilder<'a, Schema, select::SelectInitial>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let sql = self.sql.clone().append(crate::helpers::select(columns));
        select::SelectBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    pub fn with<Q, C>(&self, cte: C) -> QueryBuilder<'a, Schema, CTEInit>
    where
        Q: ToSQL<'a, SQLiteValue<'a>>,
        C: AsRef<drizzle_core::expressions::DefinedCTE<'a, SQLiteValue<'a>, Q>>,
    {
        let sql = self
            .sql
            .clone()
            .append_raw(", ")
            .append(cte.as_ref().definition());
        QueryBuilder {
            sql,
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
    pub fn insert<Table>(
        &self,
        table: Table,
    ) -> insert::InsertBuilder<'a, Schema, insert::InsertInitial, Table>
    where
        Table: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>>,
    {
        let sql = crate::helpers::insert(table);

        insert::InsertBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    pub fn update<Table>(
        &self,
        table: Table,
    ) -> update::UpdateBuilder<'a, Schema, update::UpdateInitial, Table>
    where
        Table: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>>,
    {
        let sql = crate::helpers::update::<'a, Table, SQLiteValue<'a>>(table);

        update::UpdateBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    pub fn delete<T>(&self, table: T) -> delete::DeleteBuilder<'a, Schema, delete::DeleteInitial, T>
    where
        T: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>>,
    {
        let sql = crate::helpers::delete::<'a, T, SQLiteValue<'a>>(table);

        delete::DeleteBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    pub fn with<Q, C>(&self, cte: C) -> QueryBuilder<'a, Schema, CTEInit>
    where
        Q: ToSQL<'a, SQLiteValue<'a>>,
        C: AsRef<drizzle_core::expressions::DefinedCTE<'a, SQLiteValue<'a>, Q>>,
    {
        let sql = SQL::raw("WITH").append(cte.as_ref().definition());
        QueryBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

// Marker trait to indicate a query builder state is executable
pub trait ExecutableState {}

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
