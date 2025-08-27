// Re-export common enums and traits from core
pub use drizzle_core::{
    OrderBy, SQL, ToSQL,
    traits::{IsInSchema, SQLSchema, SQLTable},
};

// Local imports
use crate::{
    common::SQLiteSchemaType,
    traits::{SQLiteSQL, SQLiteTable},
    values::SQLiteValue,
};
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
/// ```rust,ignore
/// use drizzle_sqlite::builder::QueryBuilder;
/// use drizzle_macros::{SQLiteTable, SQLiteSchema};
///
/// #[SQLiteTable(name = "users")]
/// struct User {
///     #[integer(primary)]
///     id: i32,
///     #[text]
///     name: String,
/// }
///
/// # #[derive(SQLiteSchema)]
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
///     .from(user)
///     .where_(user.id.eq(1));
/// ```
///
/// ## Query Types
///
/// The builder supports all major SQL operations:
///
/// ### SELECT Queries
/// ```rust,ignore
/// # use drizzle_sqlite::builder::QueryBuilder;
/// # use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// # #[SQLiteTable(name = "users")] struct User { #[integer(primary)] id: i32, #[text] name: String }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// let query = builder.select(user.name).from(user);
/// let query = builder.select((user.id, user.name)).from(user).where_(user.id.gt(10));
/// ```
///
/// ### INSERT Queries
/// ```rust,ignore
/// # use drizzle_sqlite::builder::QueryBuilder;
/// # use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// # #[SQLiteTable(name = "users")] struct User { #[integer(primary)] id: i32, #[text] name: String }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// let query = builder
///     .insert(user)
///     .values([InsertUser::new("Alice")]);
/// ```
///
/// ### UPDATE Queries
/// ```rust,ignore
/// # use drizzle_sqlite::builder::QueryBuilder;
/// # use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// # #[SQLiteTable(name = "users")] struct User { #[integer(primary)] id: i32, #[text] name: String }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// let query = builder
///     .update(user)
///     .set(user.name.eq("Bob"))
///     .where_(user.id.eq(1));
/// ```
///
/// ### DELETE Queries  
/// ```rust,ignore
/// # use drizzle_sqlite::builder::QueryBuilder;
/// # use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// # #[SQLiteTable(name = "users")] struct User { #[integer(primary)] id: i32, #[text] name: String }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// let query = builder
///     .delete(user)
///     .where_(user.id.lt(10));
/// ```
///
/// ## Common Table Expressions (CTEs)
///
/// The builder supports WITH clauses for complex queries:
///
/// ```rust,ignore
/// # use drizzle_sqlite::builder::QueryBuilder;
/// # use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// # #[SQLiteTable(name = "users")] struct User { #[integer(primary)] id: i32, #[text] name: String }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// # let user_query = builder.select(user.name).from(user);
/// let cte = user_query.as_cte("active_users");
/// let query = builder
///     .with(cte)
///     .select(())
///     .from(cte);
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
    /// ```rust,ignore
    /// use drizzle_sqlite::builder::QueryBuilder;
    /// use drizzle_macros::{SQLiteTable, SQLiteSchema};
    ///
    /// #[SQLiteTable(name = "users")]
    /// struct User {
    ///     #[integer(primary)]
    ///     id: i32,
    ///     #[text]
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

impl<'a, Schema, State> QueryBuilder<'a, Schema, State>
where
    State: BuilderState,
{
    /// Begins a SELECT query with the specified columns.
    ///
    /// This method starts building a SELECT statement. You can select individual columns,
    /// multiple columns as a tuple, or use `()` to select all columns.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// # use drizzle_sqlite::builder::QueryBuilder;
    /// # use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// # use drizzle_core::ToSQL;
    /// # #[SQLiteTable(name = "users")] struct User { #[integer(primary)] id: i32, #[text] name: String }
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
    /// Begins an INSERT query for the specified table.
    ///
    /// This method starts building an INSERT statement. The table must be part of the schema
    /// and will be type-checked at compile time.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// # use drizzle_sqlite::builder::QueryBuilder;
    /// # use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// # use drizzle_core::ToSQL;
    /// # #[SQLiteTable(name = "users")] struct User { #[integer(primary)] id: i32, #[text] name: String }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// let query = builder
    ///     .insert(user)
    ///     .values([InsertUser::new("Alice")]);
    /// assert_eq!(query.to_sql().sql(), r#"INSERT INTO "users" ("name") VALUES (?)"#);
    /// ```
    pub fn insert<Table>(
        &self,
        table: Table,
    ) -> insert::InsertBuilder<'a, Schema, insert::InsertInitial, Table>
    where
        Table: IsInSchema<Schema> + SQLiteTable<'a>,
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
    /// ```rust,ignore
    /// # use drizzle_sqlite::builder::QueryBuilder;
    /// # use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// # use drizzle_core::{ToSQL, expressions::conditions::eq};
    /// # #[SQLiteTable(name = "users")] struct User { #[integer(primary)] id: i32, #[text] name: String }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// let query = builder
    ///     .update(user)
    ///     .set(eq(user.name, "Bob"))
    ///     .r#where(eq(user.id, 1));
    /// assert_eq!(query.to_sql().sql(), r#"UPDATE "users" SET "name" = ? WHERE "users"."id" = ?"#);
    /// ```
    pub fn update<Table>(
        &self,
        table: Table,
    ) -> update::UpdateBuilder<'a, Schema, update::UpdateInitial, Table>
    where
        Table: IsInSchema<Schema> + SQLiteTable<'a>,
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
    /// ```rust,ignore
    /// # use drizzle_sqlite::builder::QueryBuilder;
    /// # use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// # use drizzle_core::{ToSQL, expressions::conditions::lt};
    /// # #[SQLiteTable(name = "users")] struct User { #[integer(primary)] id: i32, #[text] name: String }
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
        Table: IsInSchema<Schema> + SQLiteTable<'a>,
    {
        let sql = crate::helpers::delete::<'a, Table, SQLiteSchemaType, SQLiteValue<'a>>(table);

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
