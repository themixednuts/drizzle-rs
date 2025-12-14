use crate::common::SQLiteSchemaType;
use crate::traits::SQLiteTable;
use crate::values::SQLiteValue;
use drizzle_core::{SQL, ToSQL};
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

/// Builds an UPDATE query specifically for SQLite.
///
/// `UpdateBuilder` provides a type-safe, fluent API for constructing UPDATE statements
/// with support for conditional updates, returning clauses, and precise column targeting.
///
/// ## Type Parameters
///
/// - `Schema`: The database schema type, ensuring only valid tables can be referenced
/// - `State`: The current builder state, enforcing proper query construction order
/// - `Table`: The table being updated
///
/// ## Query Building Flow
///
/// 1. Start with `QueryBuilder::update(table)` to specify the target table
/// 2. Add `set()` to specify which columns to update and their new values
/// 3. Optionally add `where()` to limit which rows are updated
/// 4. Optionally add `returning()` to get updated values back
///
/// ## Basic Usage
///
/// ```rust
/// use drizzle_sqlite::prelude::*;
/// use drizzle_core::expressions::conditions::eq;
/// use drizzle_sqlite::builder::QueryBuilder;
/// use drizzle_macros::{SQLiteTable, SQLiteSchema};
///
/// #[SQLiteTable(name = "users")]
/// struct User {
///     #[column(primary)]
///     id: i32,
///     name: String,
///     email: Option<String>,
/// }
///
/// #[derive(SQLiteSchema)]
/// struct Schema {
///     user: User,
/// }
///
/// let builder = QueryBuilder::new::<Schema>();
/// let Schema { user } = Schema::new();
///
/// // Basic UPDATE
/// let query = builder
///     .update(user)
///     .set(UpdateUser::default().with_name("Alice Updated"))
///     .r#where(eq(user.id, 1));
/// assert_eq!(
///     query.to_sql().sql(),
///     r#"UPDATE "users" SET "name" = ? WHERE "users"."id" = ?"#
/// );
/// ```
///
/// ## Advanced Updates
///
/// ### Multiple Column Updates
/// ```rust,no_run
/// # mod drizzle {
/// #     pub mod core { pub use drizzle_core::*; }
/// #     pub mod error { pub use drizzle_core::error::*; }
/// #     pub mod sqlite { pub use drizzle_sqlite::*; }
/// # }
/// # use drizzle::sqlite::prelude::*;
/// # use drizzle::core::expressions::conditions::eq;
/// # use drizzle::sqlite::builder::QueryBuilder;
/// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String, email: Option<String> }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// let query = builder
///     .update(user)
///     .set(UpdateUser::default()
///         .with_name("Alice Updated")
///         .with_email("alice.new@example.com"))
///     .r#where(eq(user.id, 1));
/// ```
///
/// ### UPDATE with RETURNING
/// ```rust,no_run
/// # mod drizzle {
/// #     pub mod core { pub use drizzle_core::*; }
/// #     pub mod error { pub use drizzle_core::error::*; }
/// #     pub mod sqlite { pub use drizzle_sqlite::*; }
/// # }
/// # use drizzle::sqlite::prelude::*;
/// # use drizzle::core::expressions::conditions::eq;
/// # use drizzle::sqlite::builder::QueryBuilder;
/// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String, age: Option<i32> }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// let query = builder
///     .update(user)
///     .set(UpdateUser::default().with_name("Alice Updated"))
///     .r#where(eq(user.id, 1))
///     .returning((user.id, user.name));
/// ```
pub type UpdateBuilder<'a, Schema, State, Table> = super::QueryBuilder<'a, Schema, State, Table>;

//------------------------------------------------------------------------------
// Initial State Implementation
//------------------------------------------------------------------------------

impl<'a, Schema, Table> UpdateBuilder<'a, Schema, UpdateInitial, Table>
where
    Table: SQLiteTable<'a>,
{
    /// Specifies which columns to update and their new values.
    ///
    /// This method accepts update expressions that specify which columns should
    /// be modified. You can update single or multiple columns using condition
    /// functions from `drizzle_core::expressions::conditions`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # mod drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod sqlite { pub use drizzle_sqlite::*; }
    /// # }
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # use drizzle::core::{ToSQL, expressions::conditions::{eq, and}};
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String, email: Option<String> }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// // Update single column
    /// let query = builder
    ///     .update(user)
    ///     .set(UpdateUser::default().with_name("New Name"));
    /// assert_eq!(query.to_sql().sql(), r#"UPDATE "users" SET "name" = ?"#);
    ///
    /// // Update multiple columns
    /// let query = builder
    ///     .update(user)
    ///     .set(UpdateUser::default().with_name("New Name").with_email("new@example.com"));
    /// ```
    #[inline]
    pub fn set(
        self,
        values: Table::Update,
    ) -> UpdateBuilder<'a, Schema, UpdateSetClauseSet, Table> {
        let sql = crate::helpers::set::<'a, Table, SQLiteSchemaType, SQLiteValue<'a>>(values);
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
    /// Adds a WHERE clause to specify which rows to update.
    ///
    /// Without a WHERE clause, all rows in the table would be updated. This method
    /// allows you to specify conditions to limit which rows are affected by the update.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # mod drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod sqlite { pub use drizzle_sqlite::*; }
    /// # }
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::core::expressions::conditions::{eq, gt, and};
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String, age: Option<i32> }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// // Update specific row by ID
    /// let query = builder
    ///     .update(user)
    ///     .set(UpdateUser::default().with_name("Updated Name"))
    ///     .r#where(eq(user.id, 1));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"UPDATE "users" SET "name" = ? WHERE "users"."id" = ?"#
    /// );
    ///
    /// // Update multiple rows with complex condition
    /// let query = builder
    ///     .update(user)
    ///     .set(UpdateUser::default().with_name("Updated"))
    ///     .r#where(and([gt(user.id, 10), eq(user.age, 25)]));
    /// ```
    #[inline]
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
    #[inline]
    pub fn returning(
        self,
        columns: impl ToSQL<'a, SQLiteValue<'a>>,
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
        columns: impl ToSQL<'a, SQLiteValue<'a>>,
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
