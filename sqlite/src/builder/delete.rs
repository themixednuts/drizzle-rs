//! The DELETE builder: [`DeleteBuilder`] and its states.
//!
//! Start a DELETE with [`QueryBuilder::delete`](super::QueryBuilder::delete).

use crate::values::SQLiteValue;
use core::marker::PhantomData;
use drizzle_core::ToSQL;

//------------------------------------------------------------------------------
// Type State Markers
//------------------------------------------------------------------------------

pub use drizzle_core::builder::{DeleteInitial, DeleteReturningSet, DeleteWhereSet};

//------------------------------------------------------------------------------
// DeleteBuilder Definition
//------------------------------------------------------------------------------

/// A DELETE query being built for `SQLite`.
///
/// This is [`QueryBuilder`](super::QueryBuilder) in one of the `Delete*`
/// states. Start it with [`QueryBuilder::delete`](super::QueryBuilder::delete).
///
/// # Clause order
///
/// 1. `where` (required): the rows to delete. `r#where(true)` deletes every
///    row.
/// 2. Optionally [`returning`](Self::returning).
///
/// The WHERE condition and the RETURNING columns may only reference the
/// table being deleted from; other tables do not compile.
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
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #             #[cfg(feature = "rusqlite")]
/// #             pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
/// #             #[cfg(feature = "libsql")]
/// #             pub mod libsql { pub use ::libsql::{Row, Value}; }
/// #             #[cfg(feature = "turso")]
/// #             pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
/// #         pub mod prelude {
/// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #             pub use drizzle_core::*;
/// #         }
/// #     }
/// # }
/// use drizzle::sqlite::prelude::*;
/// use drizzle::core::expr::{eq, lt};
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
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #             #[cfg(feature = "rusqlite")]
/// #             pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
/// #             #[cfg(feature = "libsql")]
/// #             pub mod libsql { pub use ::libsql::{Row, Value}; }
/// #             #[cfg(feature = "turso")]
/// #             pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
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
///     .delete(user)
///     .r#where(eq(user.id, 1))
///     .returning((user.id, user.name));
/// assert_eq!(
///     query.to_sql().sql(),
///     r#"DELETE FROM "users" WHERE "users"."id" = ? RETURNING "users"."id", "users"."name""#
/// );
/// ```
///
/// `r#where(true)` deletes every row:
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
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #             #[cfg(feature = "rusqlite")]
/// #             pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
/// #             #[cfg(feature = "libsql")]
/// #             pub mod libsql { pub use ::libsql::{Row, Value}; }
/// #             #[cfg(feature = "turso")]
/// #             pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
/// #         pub mod prelude {
/// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #             pub use drizzle_core::*;
/// #         }
/// #     }
/// # }
/// # use drizzle::sqlite::prelude::*;
/// # use drizzle::sqlite::builder::QueryBuilder;
/// # #[SQLiteTable(name = "logs")] struct Log { #[column(primary)] id: i32, message: String }
/// # #[derive(SQLiteSchema)] struct Schema { log: Log }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { log } = Schema::new();
/// let query = builder.delete(log).r#where(true);
/// assert_eq!(query.to_sql().sql(), r#"DELETE FROM "logs" WHERE ?"#);
/// ```
pub type DeleteBuilder<'a, Schema, State, Table, Marker = (), Row = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row>;

type ReturningMarker<Table, Columns> = drizzle_core::Scoped<
    <Columns as drizzle_core::IntoSelectTarget>::Marker,
    drizzle_core::Cons<Table, drizzle_core::Nil>,
>;

type ReturningRow<Table, Columns> =
    <<Columns as drizzle_core::IntoSelectTarget>::Marker as drizzle_core::ResolveRow<Table>>::Row;

type ReturningBuilder<'a, S, T, Columns> = DeleteBuilder<
    'a,
    S,
    DeleteReturningSet,
    T,
    ReturningMarker<T, Columns>,
    ReturningRow<T, Columns>,
>;

//------------------------------------------------------------------------------
// Initial State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> DeleteBuilder<'a, S, DeleteInitial, T> {
    /// Adds a WHERE clause that picks the rows to delete.
    ///
    /// A DELETE runs only with a WHERE clause, so a forgotten condition does
    /// not empty the table; `r#where(true)` deletes every row. The condition
    /// must be a boolean expression over the target table's columns.
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
    /// # use drizzle::core::expr::{eq, gt, and, or};
    /// # use drizzle::sqlite::builder::QueryBuilder;
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
    ///     .r#where(and(
    ///         gt(user.id, 100),
    ///         or(eq(user.name, "test"), eq(user.age, 0))
    ///     ));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"DELETE FROM "users" WHERE ("users"."id" > ? AND ("users"."name" = ? OR "users"."age" = ?))"#
    /// );
    /// ```
    #[inline]
    pub fn r#where<E, ScopeProof>(self, condition: E) -> DeleteBuilder<'a, S, DeleteWhereSet, T>
    where
        E: drizzle_core::expr::ExprSources,
        E::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
        E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        let where_sql = crate::helpers::r#where(condition);
        DeleteBuilder {
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

impl<'a, S, T> DeleteBuilder<'a, S, DeleteWhereSet, T> {
    /// Adds a RETURNING clause after WHERE. See
    /// [`returning`](DeleteBuilder::returning).
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
        DeleteBuilder {
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
