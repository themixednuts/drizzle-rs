use drizzle_core::Token;
// Re-export common enums and traits from core
pub use drizzle_core::builder::{BuilderInit, ExecutableState, OrderByClause};
pub use drizzle_core::{
    OrderBy, SQL, ToSQL,
    traits::{SQLSchema, SQLTable},
};

// Local imports
use crate::{
    common::SQLiteSchemaType,
    traits::{SQLiteSQL, SQLiteTable},
    values::SQLiteValue,
};
use std::{fmt::Debug, marker::PhantomData};

// Import modules - these provide specific builder types
pub mod cte;
pub mod delete;
pub mod insert;
pub mod prepared;
pub mod select;
pub mod update;

// Re-export CTE types
pub use cte::{CTEDefinition, CTEView};

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

#[derive(Debug, Clone)]
pub struct CTEInit;

impl ExecutableState for CTEInit {}

/// Main query builder for SQLite operations.
///
/// `QueryBuilder` provides a type-safe, fluent API for building SQL queries. It uses compile-time
/// type checking to ensure queries are valid and properly structured.
///
/// ## Type Parameters
///
/// - `Schema`: The database schema type, ensuring queries only reference valid tables
/// - `State`: The current builder state, enforcing proper query construction order
/// - `Table`: The table type being operated on (for single-table operations)
///
/// ## Basic Usage
///
/// ```
/// # mod drizzle {
/// #     pub mod core { pub use drizzle_core::*; }
/// #     pub mod error { pub use drizzle_core::error::*; }
/// #     pub mod types { pub use drizzle_types::*; }
/// #     pub mod migrations { pub use drizzle_migrations::*; }
/// #     pub use drizzle_types::Dialect;
/// #     pub mod sqlite {
/// #         pub use drizzle_sqlite::*;
/// #         pub mod prelude {
/// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #             pub use drizzle_core::*;
/// #         }
/// #     }
/// # }
/// use drizzle::sqlite::prelude::*;
/// use drizzle::sqlite::builder::QueryBuilder;
///
/// #[SQLiteTable(name = "users")]
/// struct User {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// #[derive(SQLiteSchema)]
/// struct Schema {
///     user: User,
/// }
///
/// // Create a query builder for your schema
/// let builder = QueryBuilder::new::<Schema>();
/// let Schema { user } = Schema::new();
///
/// // Build queries using the fluent API
/// let query = builder
///     .select(user.name)
///     .from(user);
/// assert_eq!(query.to_sql().sql(), r#"SELECT "users"."name" FROM "users""#);
/// ```
///
/// ## Query Types
///
/// The builder supports all major SQL operations:
///
/// ### SELECT Queries
/// ```rust,no_run
/// # mod drizzle {
/// #     pub mod core { pub use drizzle_core::*; }
/// #     pub mod error { pub use drizzle_core::error::*; }
/// #     pub mod types { pub use drizzle_types::*; }
/// #     pub mod migrations { pub use drizzle_migrations::*; }
/// #     pub use drizzle_types::Dialect;
/// #     pub mod sqlite {
/// #         pub use drizzle_sqlite::*;
/// #         pub mod prelude {
/// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #             pub use drizzle_core::*;
/// #         }
/// #     }
/// # }
/// # use drizzle::sqlite::prelude::*;
/// # use drizzle::core::expr::gt;
/// # use drizzle::sqlite::builder::QueryBuilder;
/// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// let query = builder.select(user.name).from(user);
/// let query = builder.select((user.id, user.name)).from(user).r#where(gt(user.id, 10));
/// ```
///
/// ### INSERT Queries
/// ```rust,no_run
/// # mod drizzle {
/// #     pub mod core { pub use drizzle_core::*; }
/// #     pub mod error { pub use drizzle_core::error::*; }
/// #     pub mod types { pub use drizzle_types::*; }
/// #     pub mod migrations { pub use drizzle_migrations::*; }
/// #     pub use drizzle_types::Dialect;
/// #     pub mod sqlite {
/// #         pub use drizzle_sqlite::*;
/// #         pub mod prelude {
/// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #             pub use drizzle_core::*;
/// #         }
/// #     }
/// # }
/// # use drizzle::sqlite::prelude::*;
/// # use drizzle::sqlite::builder::QueryBuilder;
/// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// let query = builder
///     .insert(user)
///     .values([InsertUser::new("Alice")]);
/// ```
///
/// ### UPDATE Queries
/// ```rust,no_run
/// # mod drizzle {
/// #     pub mod core { pub use drizzle_core::*; }
/// #     pub mod error { pub use drizzle_core::error::*; }
/// #     pub mod types { pub use drizzle_types::*; }
/// #     pub mod migrations { pub use drizzle_migrations::*; }
/// #     pub use drizzle_types::Dialect;
/// #     pub mod sqlite {
/// #         pub use drizzle_sqlite::*;
/// #         pub mod prelude {
/// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #             pub use drizzle_core::*;
/// #         }
/// #     }
/// # }
/// # use drizzle::sqlite::prelude::*;
/// # use drizzle::core::expr::eq;
/// # use drizzle::sqlite::builder::QueryBuilder;
/// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// let query = builder
///     .update(user)
///     .set(UpdateUser::default().with_name("Bob"))
///     .r#where(eq(user.id, 1));
/// ```
///
/// ### DELETE Queries  
/// ```rust,no_run
/// # mod drizzle {
/// #     pub mod core { pub use drizzle_core::*; }
/// #     pub mod error { pub use drizzle_core::error::*; }
/// #     pub mod types { pub use drizzle_types::*; }
/// #     pub mod migrations { pub use drizzle_migrations::*; }
/// #     pub use drizzle_types::Dialect;
/// #     pub mod sqlite {
/// #         pub use drizzle_sqlite::*;
/// #         pub mod prelude {
/// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #             pub use drizzle_core::*;
/// #         }
/// #     }
/// # }
/// # use drizzle::sqlite::prelude::*;
/// # use drizzle::core::expr::lt;
/// # use drizzle::sqlite::builder::QueryBuilder;
/// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// let query = builder
///     .delete(user)
///     .r#where(lt(user.id, 10));
/// ```
///
/// ## Common Table Expressions (CTEs)
///
/// The builder supports WITH clauses for complex queries with typed field access:
///
/// ```rust
/// # mod drizzle {
/// #     pub mod core { pub use drizzle_core::*; }
/// #     pub mod error { pub use drizzle_core::error::*; }
/// #     pub mod types { pub use drizzle_types::*; }
/// #     pub mod migrations { pub use drizzle_migrations::*; }
/// #     pub use drizzle_types::Dialect;
/// #     pub mod sqlite {
/// #         pub use drizzle_sqlite::*;
/// #         pub mod prelude {
/// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #             pub use drizzle_core::*;
/// #         }
/// #     }
/// # }
/// # use drizzle::sqlite::prelude::*;
/// # use drizzle::sqlite::builder::QueryBuilder;
/// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// // Create a CTE with typed field access using .as_cte()
/// let active_users = builder
///     .select((user.id, user.name))
///     .from(user)
///     .as_cte("active_users");
///
/// // Use the CTE with typed column access via Deref
/// let query = builder
///     .with(&active_users)
///     .select(active_users.name)  // Typed field access!
///     .from(&active_users);
/// assert_eq!(
///     query.to_sql().sql(),
///     r#"WITH active_users AS (SELECT "users"."id", "users"."name" FROM "users") SELECT "active_users"."name" FROM "active_users""#
/// );
/// ```
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
    fn to_sql(&self) -> SQLiteSQL<'a> {
        self.sql.clone()
    }
}

impl<'a> QueryBuilder<'a> {
    /// Creates a new query builder for the given schema type.
    ///
    /// This is the entry point for building SQL queries. The schema type parameter
    /// ensures that only valid tables from your schema can be used in queries.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # mod drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub mod sqlite {
    /// #         pub use drizzle_sqlite::*;
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// use drizzle::sqlite::prelude::*;
    /// use drizzle::sqlite::builder::QueryBuilder;
    ///
    /// #[SQLiteTable(name = "users")]
    /// struct User {
    ///     #[column(primary)]
    ///     id: i32,
    ///     name: String,
    /// }
    ///
    /// #[derive(SQLiteSchema)]
    /// struct MySchema {
    ///     user: User,
    /// }
    ///
    /// let builder = QueryBuilder::new::<MySchema>();
    /// ```
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
    /// Begins a SELECT query with the specified columns.
    ///
    /// This method starts building a SELECT statement. You can select individual columns,
    /// multiple columns as a tuple, or use `()` to select all columns.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # mod drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub mod sqlite {
    /// #         pub use drizzle_sqlite::*;
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// // Select a single column
    /// let query = builder.select(user.name).from(user);
    /// assert_eq!(query.to_sql().sql(), r#"SELECT "users"."name" FROM "users""#);
    ///
    /// // Select multiple columns
    /// let query = builder.select((user.id, user.name)).from(user);
    /// assert_eq!(query.to_sql().sql(), r#"SELECT "users"."id", "users"."name" FROM "users""#);
    /// ```
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

    /// Begins a SELECT DISTINCT query with the specified columns.
    ///
    /// SELECT DISTINCT removes duplicate rows from the result set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # mod drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub mod sqlite {
    /// #         pub use drizzle_sqlite::*;
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// let query = builder.select_distinct(user.name).from(user);
    /// assert_eq!(query.to_sql().sql(), r#"SELECT DISTINCT "users"."name" FROM "users""#);
    /// ```
    pub fn select_distinct<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let sql = crate::helpers::select_distinct(columns);
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

    /// Begins a SELECT DISTINCT query with the specified columns after a CTE.
    pub fn select_distinct<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
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

    /// Begins an INSERT query after a CTE.
    pub fn insert<Table>(
        &self,
        table: Table,
    ) -> insert::InsertBuilder<'a, Schema, insert::InsertInitial, Table>
    where
        Table: SQLiteTable<'a>,
    {
        let sql = self.sql.clone().append(crate::helpers::insert::<
            'a,
            Table,
            SQLiteSchemaType,
            SQLiteValue<'a>,
        >(table));

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
        Table: SQLiteTable<'a>,
    {
        let sql = self.sql.clone().append(crate::helpers::update::<
            'a,
            Table,
            SQLiteSchemaType,
            SQLiteValue<'a>,
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
        Table: SQLiteTable<'a>,
    {
        let sql = self.sql.clone().append(crate::helpers::delete::<
            'a,
            Table,
            SQLiteSchemaType,
            SQLiteValue<'a>,
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
    /// Begins an INSERT query for the specified table.
    ///
    /// This method starts building an INSERT statement. The table must be part of the schema
    /// and will be type-checked at compile time.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # mod drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub mod sqlite {
    /// #         pub use drizzle_sqlite::*;
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// let query = builder
    ///     .insert(user)
    ///     .values([InsertUser::new("Alice")]);
    /// assert_eq!(query.to_sql().sql(), r#"INSERT INTO "users" (name) VALUES (?)"#);
    /// ```
    pub fn insert<Table>(
        &self,
        table: Table,
    ) -> insert::InsertBuilder<'a, Schema, insert::InsertInitial, Table>
    where
        Table: SQLiteTable<'a>,
    {
        let sql = crate::helpers::insert(table);

        insert::InsertBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Begins an UPDATE query for the specified table.
    ///
    /// This method starts building an UPDATE statement. The table must be part of the schema
    /// and will be type-checked at compile time.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # mod drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub mod sqlite {
    /// #         pub use drizzle_sqlite::*;
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::core::expr::eq;
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// let query = builder
    ///     .update(user)
    ///     .set(UpdateUser::default().with_name("Bob"))
    ///     .r#where(eq(user.id, 1));
    /// assert_eq!(query.to_sql().sql(), r#"UPDATE "users" SET "name" = ? WHERE "users"."id" = ?"#);
    /// ```
    pub fn update<Table>(
        &self,
        table: Table,
    ) -> update::UpdateBuilder<'a, Schema, update::UpdateInitial, Table>
    where
        Table: SQLiteTable<'a>,
    {
        let sql = crate::helpers::update::<'a, Table, SQLiteSchemaType, SQLiteValue<'a>>(table);

        update::UpdateBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Begins a DELETE query for the specified table.
    ///
    /// This method starts building a DELETE statement. The table must be part of the schema
    /// and will be type-checked at compile time.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # mod drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub mod sqlite {
    /// #         pub use drizzle_sqlite::*;
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::core::expr::lt;
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// let query = builder
    ///     .delete(user)
    ///     .r#where(lt(user.id, 10));
    /// assert_eq!(query.to_sql().sql(), r#"DELETE FROM "users" WHERE "users"."id" < ?"#);
    /// ```
    pub fn delete<Table>(
        &self,
        table: Table,
    ) -> delete::DeleteBuilder<'a, Schema, delete::DeleteInitial, Table>
    where
        Table: SQLiteTable<'a>,
    {
        let sql = crate::helpers::delete::<'a, Table, SQLiteSchemaType, SQLiteValue<'a>>(table);

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
