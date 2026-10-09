//! The UPDATE builder: [`UpdateBuilder`] and its states.
//!
//! Start an UPDATE with [`QueryBuilder::update`](super::QueryBuilder::update).

use crate::common::SQLiteSchemaType;
use crate::traits::SQLiteTable;
use crate::values::SQLiteValue;
use core::marker::PhantomData;
use drizzle_core::ToSQL;

//------------------------------------------------------------------------------
// Type State Markers
//------------------------------------------------------------------------------

pub use drizzle_core::builder::{
    UpdateInitial, UpdateReturningSet, UpdateSetClauseSet, UpdateWhereSet,
};

//------------------------------------------------------------------------------
// UpdateBuilder Definition
//------------------------------------------------------------------------------

/// An UPDATE query being built for `SQLite`.
///
/// This is [`QueryBuilder`](super::QueryBuilder) in one of the `Update*`
/// states. Start it with [`QueryBuilder::update`](super::QueryBuilder::update).
///
/// # Clause order
///
/// 1. [`set`](Self::set) (required before the query can run).
/// 2. `where` (required): the rows to update. `r#where(true)` updates every
///    row.
/// 3. Optionally [`returning`](Self::returning).
///
/// The WHERE condition and the RETURNING columns may only reference the
/// table being updated; other tables do not compile.
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
/// #     pub use drizzle_types as ddl;
/// #     pub mod sqlite {
/// #         pub use drizzle_sqlite::*;
/// #         #[cfg(feature = "rusqlite")]
/// #         pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
/// #         #[cfg(feature = "libsql")]
/// #         pub mod libsql { pub use ::libsql::{Row, Value}; }
/// #         #[cfg(feature = "turso")]
/// #         pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
/// #         pub mod prelude {
/// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #             pub use drizzle_core::*;
/// #         }
/// #     }
/// # }
/// use drizzle::sqlite::prelude::*;
/// use drizzle::core::expr::eq;
/// use drizzle::sqlite::builder::QueryBuilder;
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
/// Several columns at once:
///
/// ```rust
/// # mod drizzle {
/// #     pub mod core { pub use drizzle_core::*; }
/// #     pub mod error { pub use drizzle_core::error::*; }
/// #     pub mod types { pub use drizzle_types::*; }
/// #     pub mod migrations { pub use drizzle_migrations::*; }
/// #     pub use drizzle_types::Dialect;
/// #     pub use drizzle_types as ddl;
/// #     pub mod sqlite {
/// #         pub use drizzle_sqlite::*;
/// #         #[cfg(feature = "rusqlite")]
/// #         pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
/// #         #[cfg(feature = "libsql")]
/// #         pub mod libsql { pub use ::libsql::{Row, Value}; }
/// #         #[cfg(feature = "turso")]
/// #         pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
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
/// assert_eq!(
///     query.to_sql().sql(),
///     r#"UPDATE "users" SET "name" = ?, "email" = ? WHERE "users"."id" = ?"#
/// );
/// ```
///
/// With RETURNING:
///
/// ```rust
/// # mod drizzle {
/// #     pub mod core { pub use drizzle_core::*; }
/// #     pub mod error { pub use drizzle_core::error::*; }
/// #     pub mod types { pub use drizzle_types::*; }
/// #     pub mod migrations { pub use drizzle_migrations::*; }
/// #     pub use drizzle_types::Dialect;
/// #     pub use drizzle_types as ddl;
/// #     pub mod sqlite {
/// #         pub use drizzle_sqlite::*;
/// #         #[cfg(feature = "rusqlite")]
/// #         pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
/// #         #[cfg(feature = "libsql")]
/// #         pub mod libsql { pub use ::libsql::{Row, Value}; }
/// #         #[cfg(feature = "turso")]
/// #         pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
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
/// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String, age: Option<i32> }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// let query = builder
///     .update(user)
///     .set(UpdateUser::default().with_name("Alice Updated"))
///     .r#where(eq(user.id, 1))
///     .returning((user.id, user.name));
/// assert_eq!(
///     query.to_sql().sql(),
///     r#"UPDATE "users" SET "name" = ? WHERE "users"."id" = ? RETURNING "users"."id", "users"."name""#
/// );
/// ```
pub type UpdateBuilder<'a, Schema, State, Table, Marker = (), Row = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row>;

type ReturningMarker<Table, Columns> = drizzle_core::Scoped<
    <Columns as drizzle_core::IntoSelectTarget>::Marker,
    drizzle_core::Cons<Table, drizzle_core::Nil>,
>;

type ReturningRow<Table, Columns> =
    <<Columns as drizzle_core::IntoSelectTarget>::Marker as drizzle_core::ResolveRow<Table>>::Row;

type ReturningBuilder<'a, S, T, Columns> = UpdateBuilder<
    'a,
    S,
    UpdateReturningSet,
    T,
    ReturningMarker<T, Columns>,
    ReturningRow<T, Columns>,
>;

//------------------------------------------------------------------------------
// Initial State Implementation
//------------------------------------------------------------------------------

impl<'a, Schema, Table> UpdateBuilder<'a, Schema, UpdateInitial, Table>
where
    Table: SQLiteTable<'a>,
{
    /// Sets the columns to change, using the table's generated update model.
    ///
    /// Start from `UpdateX::default()` and call a `with_*` setter for each
    /// column to change. Columns you do not set are left as they are.
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
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod sqlite {
    /// #         pub use drizzle_sqlite::*;
    /// #         #[cfg(feature = "rusqlite")]
    /// #         pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
    /// #         #[cfg(feature = "libsql")]
    /// #         pub mod libsql { pub use ::libsql::{Row, Value}; }
    /// #         #[cfg(feature = "turso")]
    /// #         pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # use drizzle::core::{ToSQL, expr::{eq, and}};
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
    /// assert_eq!(query.to_sql().sql(), r#"UPDATE "users" SET "name" = ?, "email" = ?"#);
    /// ```
    #[inline]
    pub fn set(
        self,
        values: Table::Update,
    ) -> UpdateBuilder<'a, Schema, UpdateSetClauseSet, Table> {
        let sql = crate::helpers::set::<Table, SQLiteSchemaType, SQLiteValue<'a>>(&values);
        drop(values);
        UpdateBuilder {
            sql: self.sql.append(sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-SET Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> UpdateBuilder<'a, S, UpdateSetClauseSet, T> {
    /// Adds a WHERE clause that picks the rows to update.
    ///
    /// An UPDATE runs only with a WHERE clause, so a forgotten condition does
    /// not rewrite the whole table; `r#where(true)` updates every row. The
    /// condition must be a boolean expression over the updated table's
    /// columns.
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
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod sqlite {
    /// #         pub use drizzle_sqlite::*;
    /// #         #[cfg(feature = "rusqlite")]
    /// #         pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
    /// #         #[cfg(feature = "libsql")]
    /// #         pub mod libsql { pub use ::libsql::{Row, Value}; }
    /// #         #[cfg(feature = "turso")]
    /// #         pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::core::expr::{eq, gt, and};
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
    ///     .r#where(and(gt(user.id, 10), eq(user.age, 25)));
    /// ```
    #[inline]
    pub fn r#where<E, ScopeProof>(self, condition: E) -> UpdateBuilder<'a, S, UpdateWhereSet, T>
    where
        E: drizzle_core::expr::ExprSources,
        E::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
        E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        let where_sql = crate::helpers::r#where(condition);
        UpdateBuilder {
            sql: self.sql.append(where_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-WHERE Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> UpdateBuilder<'a, S, UpdateWhereSet, T> {
    /// Adds a RETURNING clause after WHERE. See
    /// [`returning`](UpdateBuilder::returning).
    #[inline]
    pub fn returning<Columns, ScopeProof>(
        self,
        columns: Columns,
    ) -> ReturningBuilder<'a, S, T, Columns>
    where
        Columns: drizzle_core::expr::ExprSources,
        Columns::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
        Columns: ToSQL<'a, SQLiteValue<'a>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<T>,
    {
        let returning_sql = crate::helpers::returning(columns);
        UpdateBuilder {
            sql: self.sql.append(returning_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }
}
