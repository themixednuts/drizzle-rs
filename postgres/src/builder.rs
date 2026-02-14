use drizzle_core::Token;
// Re-export common enums and traits from core
pub use drizzle_core::builder::{BuilderInit, ExecutableState, OrderByClause};
pub use drizzle_core::{
    OrderBy, SQL, ToSQL,
    traits::{SQLSchema, SQLTable},
};

// Local imports
use crate::{common::PostgresSchemaType, traits::PostgresTable, values::PostgresValue};
use std::{fmt::Debug, marker::PhantomData};

// Import modules - these provide specific builder types
pub mod cte;
pub mod delete;
pub mod insert;
pub mod prepared;
pub mod refresh;
pub mod select;
pub mod update;

// Re-export CTE types
pub use cte::{CTEDefinition, CTEView};

// Export state markers for easier use
pub use delete::{DeleteInitial, DeleteReturningSet, DeleteWhereSet};
pub use insert::{
    InsertDoUpdateSet, InsertInitial, InsertOnConflictSet, InsertReturningSet, InsertValuesSet,
    OnConflictBuilder,
};
pub use refresh::{
    RefreshConcurrently, RefreshInitial, RefreshMaterializedView, RefreshWithNoData,
    refresh_materialized_view,
};
pub use select::{
    SelectForSet, SelectFromSet, SelectGroupSet, SelectInitial, SelectJoinSet, SelectLimitSet,
    SelectOffsetSet, SelectOrderSet, SelectWhereSet,
};
pub use update::{
    UpdateFromSet, UpdateInitial, UpdateReturningSet, UpdateSetClauseSet, UpdateWhereSet,
};

// Re-export SQLViewInfo for convenience when using refresh_materialized_view
pub use drizzle_core::traits::SQLViewInfo;

#[derive(Debug, Clone)]
pub struct CTEInit;

impl ExecutableState for CTEInit {}

/// Main query builder for PostgreSQL
///
/// The `S` type parameter represents the schema type, which is used
/// to ensure type safety when building queries.
#[derive(Debug, Clone, Default)]
pub struct QueryBuilder<'a, Schema = (), State = (), Table = ()> {
    pub sql: SQL<'a, PostgresValue<'a>>,
    schema: PhantomData<Schema>,
    state: PhantomData<State>,
    table: PhantomData<Table>,
}

//------------------------------------------------------------------------------
// QueryBuilder Implementation
//------------------------------------------------------------------------------

impl<'a, Schema, State, Table> ToSQL<'a, PostgresValue<'a>>
    for QueryBuilder<'a, Schema, State, Table>
{
    fn to_sql(&self) -> SQL<'a, PostgresValue<'a>> {
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

impl<'a, Schema> QueryBuilder<'a, Schema, BuilderInit> {
    pub fn select<T>(&self, columns: T) -> select::SelectBuilder<'a, Schema, select::SelectInitial>
    where
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let sql = crate::helpers::select(columns);
        select::SelectBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Begins a SELECT DISTINCT query with the specified columns.
    ///
    /// SELECT DISTINCT removes duplicate rows from the result set.
    pub fn select_distinct<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial>
    where
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let sql = crate::helpers::select_distinct(columns);
        select::SelectBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Begins a SELECT DISTINCT ON query with the specified columns.
    pub fn select_distinct_on<On, Columns>(
        &self,
        on: On,
        columns: Columns,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial>
    where
        On: ToSQL<'a, PostgresValue<'a>>,
        Columns: ToSQL<'a, PostgresValue<'a>>,
    {
        let sql = crate::helpers::select_distinct_on(on, columns);
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
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let sql = self.sql.clone().append(crate::helpers::select(columns));
        select::SelectBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Begins a SELECT DISTINCT query with the specified columns after a CTE.
    pub fn select_distinct<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial>
    where
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let sql = self
            .sql
            .clone()
            .append(crate::helpers::select_distinct(columns));
        select::SelectBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Begins a SELECT DISTINCT ON query with the specified columns after a CTE.
    pub fn select_distinct_on<On, Columns>(
        &self,
        on: On,
        columns: Columns,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial>
    where
        On: ToSQL<'a, PostgresValue<'a>>,
        Columns: ToSQL<'a, PostgresValue<'a>>,
    {
        let sql = self
            .sql
            .clone()
            .append(crate::helpers::select_distinct_on(on, columns));
        select::SelectBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Begins an INSERT query after a CTE.
    pub fn insert<Table>(
        &self,
        table: Table,
    ) -> insert::InsertBuilder<'a, Schema, insert::InsertInitial, Table>
    where
        Table: PostgresTable<'a>,
    {
        let sql = self.sql.clone().append(crate::helpers::insert(table));

        insert::InsertBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Begins an UPDATE query after a CTE.
    pub fn update<Table>(
        &self,
        table: Table,
    ) -> update::UpdateBuilder<'a, Schema, update::UpdateInitial, Table>
    where
        Table: PostgresTable<'a>,
    {
        let sql = self.sql.clone().append(crate::helpers::update::<
            'a,
            Table,
            PostgresSchemaType,
            PostgresValue<'a>,
        >(table));

        update::UpdateBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Begins a DELETE query after a CTE.
    pub fn delete<Table>(
        &self,
        table: Table,
    ) -> delete::DeleteBuilder<'a, Schema, delete::DeleteInitial, Table>
    where
        Table: PostgresTable<'a>,
    {
        let sql = self.sql.clone().append(crate::helpers::delete::<
            'a,
            Table,
            PostgresSchemaType,
            PostgresValue<'a>,
        >(table));

        delete::DeleteBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    pub fn with<C>(&self, cte: C) -> QueryBuilder<'a, Schema, CTEInit>
    where
        C: CTEDefinition<'a>,
    {
        let sql = self
            .sql
            .clone()
            .push(Token::COMMA)
            .append(cte.cte_definition());
        QueryBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

impl<'a, Schema> QueryBuilder<'a, Schema, BuilderInit> {
    pub fn insert<Table>(
        &self,
        table: Table,
    ) -> insert::InsertBuilder<'a, Schema, insert::InsertInitial, Table>
    where
        Table: PostgresTable<'a>,
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
        Table: PostgresTable<'a>,
    {
        let sql = crate::helpers::update::<'a, Table, PostgresSchemaType, PostgresValue<'a>>(table);

        update::UpdateBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    pub fn delete<Table>(
        &self,
        table: Table,
    ) -> delete::DeleteBuilder<'a, Schema, delete::DeleteInitial, Table>
    where
        Table: PostgresTable<'a>,
    {
        let sql = crate::helpers::delete::<'a, Table, PostgresSchemaType, PostgresValue<'a>>(table);

        delete::DeleteBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    pub fn with<C>(&self, cte: C) -> QueryBuilder<'a, Schema, CTEInit>
    where
        C: CTEDefinition<'a>,
    {
        let sql = SQL::from(Token::WITH).append(cte.cte_definition());
        QueryBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

// Marker trait to indicate a query builder state is executable
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder_new() {
        let qb = QueryBuilder::new::<()>();
        let sql = qb.to_sql();
        assert_eq!(sql.sql(), "");
        assert_eq!(sql.params().count(), 0);
    }

    #[test]
    fn test_builder_init_type() {
        let _state = BuilderInit;
    }
}
