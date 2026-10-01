use crate::common::PostgresSchemaType;
use crate::values::PostgresValue;
use core::marker::PhantomData;
use drizzle_core::{SQLTable, ToSQL};

// Import the ExecutableState trait
use super::ExecutableState;

//------------------------------------------------------------------------------
// Type State Markers
//------------------------------------------------------------------------------

pub use drizzle_core::builder::{
    UpdateInitial, UpdateReturningSet, UpdateSetClauseSet, UpdateWhereSet,
};

/// Builder state after `UPDATE ... SET ... FROM source`.
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateFromSet;

// Mark states that can execute update queries
impl ExecutableState for UpdateFromSet {}

//------------------------------------------------------------------------------
// UpdateBuilder Definition
//------------------------------------------------------------------------------

/// A `PostgreSQL` `UPDATE` being built: a [`QueryBuilder`](super::QueryBuilder)
/// in one of the `Update*` states.
///
/// Start with `.set(update_model)`, then optionally add `FROM`, `WHERE` and
/// `RETURNING`, in that order. Without `WHERE`, every row is updated.
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
/// use drizzle::core::expr::eq;
///
/// let query = db
///     .update(user)
///     .set(UpdateUser::default().with_name("Bob"))
///     .r#where(eq(user.id, 1))
///     .returning(user.id);
/// assert_eq!(
///     query.to_sql().sql(),
///     r#"UPDATE "users" SET "name" = $1 WHERE "users"."id" = $2 RETURNING "users"."id""#
/// );
/// # }
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
    Table: SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>,
{
    /// Sets the new column values from the table's `Update*` model.
    ///
    /// Only the fields set on the model (with `.with_*`) are written.
    #[inline]
    pub fn set(
        self,
        values: Table::Update,
    ) -> UpdateBuilder<'a, Schema, UpdateSetClauseSet, Table> {
        let sql = crate::helpers::set::<Table, PostgresSchemaType, PostgresValue<'a>>(&values);
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
    /// Adds `FROM source`, making another table's columns available to
    /// `WHERE` and `RETURNING`.
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
    /// use drizzle::core::expr::eq;
    ///
    /// let query = db
    ///     .update(user)
    ///     .set(UpdateUser::default().with_name("Author"))
    ///     .from(post)
    ///     .r#where(eq(post.author_id, user.id));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"UPDATE "users" SET "name" = $1 FROM "posts" WHERE "posts"."author_id" = "users"."id""#
    /// );
    /// # }
    /// ```
    #[inline]
    pub fn from<F>(
        self,
        source: F,
    ) -> UpdateBuilder<'a, S, UpdateFromSet, T, drizzle_core::Cons<F, drizzle_core::Nil>>
    where
        F: ToSQL<'a, PostgresValue<'a>> + drizzle_core::ScopeEntry,
    {
        let from_sql = crate::helpers::from(source);
        UpdateBuilder {
            sql: self.sql.append(from_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Adds a `WHERE` condition; only matching rows are updated.
    ///
    /// The condition may only use columns of the updated table; to use other
    /// tables, add [`from`](Self::from) first.
    #[inline]
    pub fn r#where<E, ScopeProof>(self, condition: E) -> UpdateBuilder<'a, S, UpdateWhereSet, T>
    where
        E: drizzle_core::expr::ExprSources,
        E::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
        E: drizzle_core::expr::Expr<'a, PostgresValue<'a>>,
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

    /// Adds `RETURNING columns`, so the statement returns the updated rows.
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

//------------------------------------------------------------------------------
// Post-FROM Implementation
//------------------------------------------------------------------------------

impl<'a, S, T, M> UpdateBuilder<'a, S, UpdateFromSet, T, M> {
    /// Adds a `WHERE` condition, which may use the updated table and the `FROM` source.
    #[inline]
    pub fn r#where<E, ScopeProof>(self, condition: E) -> UpdateBuilder<'a, S, UpdateWhereSet, T, M>
    where
        E: drizzle_core::expr::ExprSources,
        E::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, M>, ScopeProof>,
        E: drizzle_core::expr::Expr<'a, PostgresValue<'a>>,
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

    /// Adds `RETURNING columns` after `FROM`.
    #[inline]
    pub fn returning<Columns, ScopeProof>(
        self,
        columns: Columns,
    ) -> ReturningBuilder<'a, S, T, Columns>
    where
        Columns: drizzle_core::expr::ExprSources,
        Columns::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, M>, ScopeProof>,
        Columns: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
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

//------------------------------------------------------------------------------
// Post-WHERE Implementation
//------------------------------------------------------------------------------

impl<'a, S, T, M> UpdateBuilder<'a, S, UpdateWhereSet, T, M> {
    /// Adds `RETURNING columns` after `WHERE`.
    #[inline]
    pub fn returning<Columns, ScopeProof>(
        self,
        columns: Columns,
    ) -> ReturningBuilder<'a, S, T, Columns>
    where
        Columns: drizzle_core::expr::ExprSources,
        Columns::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, M>, ScopeProof>,
        Columns: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
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

#[cfg(test)]
mod tests {
    use super::*;
    use drizzle_core::{SQL, ToSQL};

    #[test]
    fn test_update_builder_creation() {
        let builder = UpdateBuilder::<(), UpdateInitial, ()> {
            sql: SQL::raw("UPDATE test"),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        };

        assert_eq!(builder.to_sql().sql(), "UPDATE test");
    }
}
