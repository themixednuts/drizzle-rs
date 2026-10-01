use crate::values::MySQLValue;
pub use drizzle_core::builder::{DeleteInitial, DeleteWhereSet};

/// Builder state after `order_by` on a `DELETE`.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeleteOrderSet;

/// Builder state after `limit` on a `DELETE`.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeleteLimitSet;

impl drizzle_core::ExecutableState for DeleteOrderSet {}
impl drizzle_core::ExecutableState for DeleteLimitSet {}

/// A `DELETE` query being built for `MySQL`.
///
/// This is [`QueryBuilder`](super::QueryBuilder) in one of the `Delete*`
/// states. Start it with [`QueryBuilder::delete`](super::QueryBuilder::delete).
///
/// # Clause order
///
/// 1. Optionally `where`. Without it, every row is deleted.
/// 2. Optionally `order_by`, then optionally `limit`.
///
/// `prepare()` is available in every state. The WHERE and ORDER BY may only
/// reference the target table; other tables do not compile. `MySQL` has no
/// `RETURNING`.
///
/// # Examples
///
/// ```rust
/// # let _ = r####"
/// # use drizzle::core::expr::{alias, count, eq, gt};
/// # use drizzle::mysql::{builder::QueryBuilder, prelude::*};
/// # #[MySQLTable(NAME = "users")]
/// # struct Users {
/// #     #[column(PRIMARY, AUTO_INCREMENT)] id: u64,
/// #     #[column(VARCHAR(255))] name: String,
/// #     #[column(DEFAULT = true)] active: bool,
/// # }
/// # #[MySQLTable(NAME = "posts")]
/// # struct Posts {
/// #     #[column(PRIMARY, AUTO_INCREMENT)] id: u64,
/// #     #[column(REFERENCES = Users::id)] user_id: u64,
/// #     title: String,
/// # }
/// # #[MySQLIndex] struct UsersNameIdx(Users::name);
/// # #[MySQLIndex] struct PostsUserIdIdx(Posts::user_id);
/// # #[derive(MySQLSchema)] struct Schema { users: Users, posts: Posts }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { users, posts } = Schema::new();
/// let query = builder
///     .delete(users)
///     .r#where(eq(users.active, false))
///     .order_by(asc(users.id))
///     .limit(4);
/// assert_eq!(
///     query.to_sql().sql(),
///     "DELETE FROM `users` WHERE `users`.`active` = ? ORDER BY `users`.`id` ASC LIMIT ?"
/// );
/// # "####;
/// ```
pub type DeleteBuilder<'a, Schema, State, Table, Marker = (), Row = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row>;

impl<'a, S, T> DeleteBuilder<'a, S, DeleteInitial, T> {
    /// Adds a WHERE clause that picks the rows to delete.
    ///
    /// The condition must be a boolean expression over the target table's
    /// columns.
    pub fn r#where<E, ScopeProof>(self, condition: E) -> DeleteBuilder<'a, S, DeleteWhereSet, T>
    where
        E: drizzle_core::expr::ExprSources,
        E::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
        E: drizzle_core::expr::Expr<'a, MySQLValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        DeleteBuilder::from_sql(self.sql.append(crate::helpers::r#where(condition)))
    }
}

mutation_builder_methods!(
    DeleteBuilder,
    prepare: [DeleteInitial, DeleteWhereSet, DeleteOrderSet, DeleteLimitSet],
    order_by: [DeleteInitial, DeleteWhereSet] => DeleteOrderSet,
    limit: [DeleteInitial, DeleteWhereSet, DeleteOrderSet] => DeleteLimitSet,
);
