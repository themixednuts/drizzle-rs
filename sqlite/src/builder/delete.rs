use crate::values::SQLiteValue;
use drizzle_core::{SQL, ToSQL};
use std::fmt::Debug;
use std::marker::PhantomData;

// Import the ExecutableState trait
use super::ExecutableState;

//------------------------------------------------------------------------------
// Type State Markers
//------------------------------------------------------------------------------

/// Marker for the initial state of DeleteBuilder
#[derive(Debug, Clone, Copy, Default)]
pub struct DeleteInitial;

/// Marker for the state after WHERE clause
#[derive(Debug, Clone, Copy, Default)]
pub struct DeleteWhereSet;

/// Marker for the state after RETURNING clause
#[derive(Debug, Clone, Copy, Default)]
pub struct DeleteReturningSet;

// Mark states that can execute delete queries
impl ExecutableState for DeleteInitial {}
impl ExecutableState for DeleteWhereSet {}
impl ExecutableState for DeleteReturningSet {}

//------------------------------------------------------------------------------
// DeleteBuilder Definition
//------------------------------------------------------------------------------

/// Builds a DELETE query specifically for SQLite.
///
/// `DeleteBuilder` provides a type-safe, fluent API for constructing DELETE statements
/// with support for conditional deletions and returning clauses.
///
/// ## Type Parameters
///
/// - `Schema`: The database schema type, ensuring only valid tables can be referenced
/// - `State`: The current builder state, enforcing proper query construction order
/// - `Table`: The table being deleted from
///
/// ## Query Building Flow
///
/// 1. Start with `QueryBuilder::delete(table)` to specify the target table
/// 2. Optionally add `where()` to specify which rows to delete
/// 3. Optionally add `returning()` to get deleted values back
///
/// ## Basic Usage
///
/// ```rust
/// use drizzle_sqlite::prelude::*;
/// use drizzle_core::expressions::conditions::{eq, lt};
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
/// // Delete specific row
/// let query = builder
///     .delete(user)
///     .r#where(eq(user.id, 1));
/// assert_eq!(query.to_sql().sql(), r#"DELETE FROM "users" WHERE "users"."id" = ?"#);
///
/// // Delete multiple rows
/// let query = builder
///     .delete(user)
///     .r#where(lt(user.id, 100));
/// assert_eq!(query.to_sql().sql(), r#"DELETE FROM "users" WHERE "users"."id" < ?"#);
/// ```
///
/// ## Advanced Deletions
///
/// ### DELETE with RETURNING
/// ```rust
/// # use drizzle_sqlite::prelude::*;
/// # use drizzle_core::expressions::conditions::eq;
/// # use drizzle_sqlite::builder::QueryBuilder;
/// # use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// let query = builder
///     .delete(user)
///     .r#where(eq(user.id, 1))
///     .returning((user.id, user.name));
/// assert_eq!(
///     query.to_sql().sql(),
///     r#"DELETE FROM "users" WHERE "users"."id" = ? RETURNING "users"."id", "users"."name""#
/// );
/// ```
///
/// ### DELETE all rows (use with caution!)
/// ```rust
/// # use drizzle_sqlite::prelude::*;
/// # use drizzle_sqlite::builder::QueryBuilder;
/// # use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// # #[SQLiteTable(name = "logs")] struct Log { #[column(primary)] id: i32, message: String }
/// # #[derive(SQLiteSchema)] struct Schema { log: Log }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { log } = Schema::new();
/// // This deletes ALL rows - be careful!
/// let query = builder.delete(log);
/// assert_eq!(query.to_sql().sql(), r#"DELETE FROM "logs""#);
/// ```
pub type DeleteBuilder<'a, Schema, State, Table> = super::QueryBuilder<'a, Schema, State, Table>;

//------------------------------------------------------------------------------
// Initial State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> DeleteBuilder<'a, S, DeleteInitial, T> {
    /// Adds a WHERE clause to specify which rows to delete.
    ///
    /// **Warning**: Without a WHERE clause, ALL rows in the table will be deleted!
    /// Always use this method unless you specifically intend to truncate the entire table.
    ///
    /// # Examples
    ///
    /// ```
    /// # use drizzle_sqlite::prelude::*;
    /// # use drizzle_core::expressions::conditions::{eq, gt, and, or};
    /// # use drizzle_sqlite::builder::QueryBuilder;
    /// # use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String, age: Option<i32> }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// // Delete specific row by ID
    /// let query = builder
    ///     .delete(user)
    ///     .r#where(eq(user.id, 1));
    /// assert_eq!(query.to_sql().sql(), r#"DELETE FROM "users" WHERE "users"."id" = ?"#);
    ///
    /// // Delete with complex conditions
    /// let query = builder
    ///     .delete(user)
    ///     .r#where(and([
    ///         gt(user.id, 100),
    ///         or([eq(user.name, "test"), eq(user.age, 0)])
    ///     ]));
    /// ```
    #[inline]
    pub fn r#where(
        self,
        condition: SQL<'a, SQLiteValue<'a>>,
    ) -> DeleteBuilder<'a, S, DeleteWhereSet, T> {
        let where_sql = crate::helpers::r#where(condition);
        DeleteBuilder {
            sql: self.sql.append(where_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Adds a RETURNING clause to the query
    #[inline]
    pub fn returning(
        self,
        columns: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> DeleteBuilder<'a, S, DeleteReturningSet, T> {
        let returning_sql = crate::helpers::returning(columns);
        DeleteBuilder {
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

impl<'a, S, T> DeleteBuilder<'a, S, DeleteWhereSet, T> {
    /// Adds a RETURNING clause after WHERE
    #[inline]
    pub fn returning(
        self,
        columns: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> DeleteBuilder<'a, S, DeleteReturningSet, T> {
        let returning_sql = crate::helpers::returning(columns);
        DeleteBuilder {
            sql: self.sql.append(returning_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}
