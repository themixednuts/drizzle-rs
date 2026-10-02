//! `DELETE` builder states and clause methods.
//!
//! [`DeleteBuilder`] is the builder returned by `QueryBuilder::delete`.
//! `WHERE` and `RETURNING` may only read the table being deleted from; this
//! is checked at the method call.

use crate::values::PostgresValue;
use core::marker::PhantomData;
use drizzle_core::ToSQL;

//------------------------------------------------------------------------------
// Type State Markers
//------------------------------------------------------------------------------

pub use drizzle_core::builder::{DeleteInitial, DeleteReturningSet, DeleteWhereSet};

//------------------------------------------------------------------------------
// DeleteBuilder Definition
//------------------------------------------------------------------------------

/// A `PostgreSQL` `DELETE` being built: a [`QueryBuilder`](super::QueryBuilder)
/// in one of the `Delete*` states.
///
/// Optionally add `WHERE`, then `RETURNING`. Without `WHERE`, every row is
/// deleted.
///
/// # Examples
///
/// ```rust
/// # extern crate self as drizzle;
/// # mod _drizzle {
/// #     pub mod core { pub use drizzle_core::*; }
/// #     pub mod error { pub use drizzle_core::error::*; }
/// #     pub mod types { pub use drizzle_types::*; }
/// #     pub mod migrations { pub use drizzle_migrations::*; }
/// #     pub use drizzle_types::Dialect;
/// #     pub use drizzle_types as ddl;
/// #     pub mod postgres {
/// #         pub mod values { pub use drizzle_postgres::values::*; }
/// #         pub mod traits { pub use drizzle_postgres::traits::*; }
/// #         pub mod common { pub use drizzle_postgres::common::*; }
/// #         pub mod attrs { pub use drizzle_postgres::attrs::*; }
/// #         pub mod builder { pub use drizzle_postgres::builder::*; }
/// #         pub mod helpers { pub use drizzle_postgres::helpers::*; }
/// #         pub mod expr { pub use drizzle_postgres::expr::*; }
/// #         pub mod types { pub use drizzle_postgres::types::*; }
/// #         #[cfg(feature = "aws-data-api")]
/// #         pub mod aws_data_api { pub use drizzle_postgres::aws_data_api::*; }
/// #         pub struct Row;
/// #         impl Row {
/// #             pub fn get<'a, I, T>(&'a self, _: I) -> T { unimplemented!() }
/// #             pub fn try_get<'a, I, T>(&'a self, _: I) -> Result<T, Box<dyn std::error::Error + Sync + Send>> { unimplemented!() }
/// #         }
/// #         pub mod prelude {
/// #             pub use drizzle_macros::{PostgresTable, PostgresSchema, PostgresIndex};
/// #             pub use drizzle_postgres::attrs::*;
/// #             pub use drizzle_postgres::common::PostgresSchemaType;
/// #             pub use drizzle_postgres::traits::{PostgresColumn, PostgresTable};
/// #             pub use drizzle_postgres::values::{PostgresInsertValue, PostgresUpdateValue, PostgresValue};
/// #             pub use drizzle_core::*;
/// #         }
/// #     }
/// # }
/// # pub use _drizzle::*;
/// # pub use const_format;
/// # fn main() {
/// # use drizzle::postgres::prelude::*;
/// # use drizzle::postgres::builder::QueryBuilder;
/// # #[PostgresTable(name = "users")]
/// # struct User {
/// #     #[column(serial, primary)]
/// #     id: i32,
/// #     name: String,
/// #     email: Option<String>,
/// # }
/// # #[PostgresTable(name = "posts")]
/// # struct Post {
/// #     #[column(serial, primary)]
/// #     id: i32,
/// #     #[column(references = User::id)]
/// #     author_id: i32,
/// #     title: String,
/// # }
/// # #[derive(PostgresSchema)]
/// # struct Schema {
/// #     user: User,
/// #     post: Post,
/// # }
/// # let db = QueryBuilder::new::<Schema>();
/// # let Schema { user, post } = Schema::new();
/// use drizzle::core::expr::gt;
///
/// let query = db
///     .delete(user)
///     .r#where(gt(user.id, 10))
///     .returning(user.id);
/// assert_eq!(
///     query.to_sql().sql(),
///     r#"DELETE FROM "users" WHERE "users"."id" > $1 RETURNING "users"."id""#
/// );
/// # }
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
    /// Adds a `WHERE` condition; only matching rows are deleted.
    ///
    /// # Compile-time checks
    ///
    /// The condition may only use columns of the table being deleted from.
    ///
    /// ```compile_fail
    /// # extern crate self as drizzle;
    /// # mod _drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod postgres {
    /// #         pub mod values { pub use drizzle_postgres::values::*; }
    /// #         pub mod traits { pub use drizzle_postgres::traits::*; }
    /// #         pub mod common { pub use drizzle_postgres::common::*; }
    /// #         pub mod attrs { pub use drizzle_postgres::attrs::*; }
    /// #         pub mod builder { pub use drizzle_postgres::builder::*; }
    /// #         pub mod helpers { pub use drizzle_postgres::helpers::*; }
    /// #         pub mod expr { pub use drizzle_postgres::expr::*; }
    /// #         pub mod types { pub use drizzle_postgres::types::*; }
    /// #         #[cfg(feature = "aws-data-api")]
    /// #         pub mod aws_data_api { pub use drizzle_postgres::aws_data_api::*; }
    /// #         pub struct Row;
    /// #         impl Row {
    /// #             pub fn get<'a, I, T>(&'a self, _: I) -> T { unimplemented!() }
    /// #             pub fn try_get<'a, I, T>(&'a self, _: I) -> Result<T, Box<dyn std::error::Error + Sync + Send>> { unimplemented!() }
    /// #         }
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{PostgresTable, PostgresSchema, PostgresIndex};
    /// #             pub use drizzle_postgres::attrs::*;
    /// #             pub use drizzle_postgres::common::PostgresSchemaType;
    /// #             pub use drizzle_postgres::traits::{PostgresColumn, PostgresTable};
    /// #             pub use drizzle_postgres::values::{PostgresInsertValue, PostgresUpdateValue, PostgresValue};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # pub use _drizzle::*;
    /// # pub use const_format;
    /// # fn main() {
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::builder::QueryBuilder;
    /// # #[PostgresTable(name = "users")]
    /// # struct User {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     name: String,
    /// #     email: Option<String>,
    /// # }
    /// # #[PostgresTable(name = "posts")]
    /// # struct Post {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     #[column(references = User::id)]
    /// #     author_id: i32,
    /// #     title: String,
    /// # }
    /// # #[derive(PostgresSchema)]
    /// # struct Schema {
    /// #     user: User,
    /// #     post: Post,
    /// # }
    /// # let db = QueryBuilder::new::<Schema>();
    /// # let Schema { user, post } = Schema::new();
    /// use drizzle::core::expr::eq;
    ///
    /// // `posts` is not part of this statement.
    /// let query = db.delete(user).r#where(eq(post.id, 1));
    /// # }
    /// ```
    #[inline]
    pub fn r#where<E, ScopeProof>(self, condition: E) -> DeleteBuilder<'a, S, DeleteWhereSet, T>
    where
        E: drizzle_core::expr::ExprSources,
        E::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
        E: drizzle_core::expr::Expr<'a, PostgresValue<'a>>,
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

    /// Adds `RETURNING columns`, so the statement returns the deleted rows.
    #[inline]
    pub fn returning<Columns, ScopeProof>(
        self,
        columns: Columns,
    ) -> ReturningBuilder<'a, S, T, Columns>
    where
        Columns: drizzle_core::expr::ExprSources,
        Columns::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
        Columns: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
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

//------------------------------------------------------------------------------
// Post-WHERE Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> DeleteBuilder<'a, S, DeleteWhereSet, T> {
    /// Adds `RETURNING columns` after `WHERE`.
    #[inline]
    pub fn returning<Columns, ScopeProof>(
        self,
        columns: Columns,
    ) -> ReturningBuilder<'a, S, T, Columns>
    where
        Columns: drizzle_core::expr::ExprSources,
        Columns::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
        Columns: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
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

#[cfg(test)]
mod tests {
    use super::*;
    use drizzle_core::{SQL, ToSQL};

    #[test]
    fn test_delete_builder_creation() {
        let builder = DeleteBuilder::<(), DeleteInitial, ()> {
            sql: SQL::raw("DELETE FROM test"),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        };

        assert_eq!(builder.to_sql().sql(), "DELETE FROM test");
    }
}
