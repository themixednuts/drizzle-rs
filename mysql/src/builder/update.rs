use crate::{common::MySQLSchemaType, values::MySQLValue};
use drizzle_core::SQLTable;

pub use drizzle_core::builder::{UpdateInitial, UpdateSetClauseSet, UpdateWhereSet};

/// Builder state after `order_by` on an `UPDATE`.
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateOrderSet;

/// Builder state after `limit` on an `UPDATE`.
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateLimitSet;

impl drizzle_core::ExecutableState for UpdateOrderSet {}
impl drizzle_core::ExecutableState for UpdateLimitSet {}

/// An `UPDATE` query being built for `MySQL`.
///
/// This is [`QueryBuilder`](super::QueryBuilder) in one of the `Update*`
/// states. Start it with [`QueryBuilder::update`](super::QueryBuilder::update).
///
/// # Clause order
///
/// 1. [`set`](Self::set) (required).
/// 2. `where` (required): the rows to update. `r#where(true)` updates every
///    row.
/// 3. Optionally `order_by`, then optionally `limit`.
///
/// `prepare()` is available once `where` is set. The WHERE and ORDER BY may
/// only reference the updated table; other tables do not compile. `MySQL`
/// has no `RETURNING`.
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
///     .update(users)
///     .set(UpdateUsers::default().with_name("Bob"))
///     .r#where(eq(users.active, true))
///     .order_by(desc(users.id))
///     .limit(1);
/// assert_eq!(
///     query.to_sql().sql(),
///     "UPDATE `users` SET `name` = ? WHERE `users`.`active` = ? ORDER BY `users`.`id` DESC LIMIT ?"
/// );
/// # "####;
/// ```
pub type UpdateBuilder<'a, Schema, State, Table, Marker = (), Row = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row>;

impl<'a, Schema, Table> UpdateBuilder<'a, Schema, UpdateInitial, Table>
where
    Table: SQLTable<'a, MySQLSchemaType, MySQLValue<'a>>,
{
    /// Sets the columns to change, using the table's generated update model.
    ///
    /// Start from `UpdateX::default()` and call a `with_*` setter for each
    /// column to change. Columns you do not set are left as they are.
    pub fn set(
        self,
        values: Table::Update,
    ) -> UpdateBuilder<'a, Schema, UpdateSetClauseSet, Table> {
        let sql = crate::helpers::set::<Table, MySQLSchemaType, MySQLValue<'a>>(&values);
        drop(values);
        UpdateBuilder::from_sql(self.sql.append(sql))
    }
}

impl<'a, S, T> UpdateBuilder<'a, S, UpdateSetClauseSet, T> {
    /// Adds a WHERE clause that picks the rows to update.
    ///
    /// The condition must be a boolean expression over the updated table's
    /// columns.
    pub fn r#where<E, ScopeProof>(self, condition: E) -> UpdateBuilder<'a, S, UpdateWhereSet, T>
    where
        E: drizzle_core::expr::ExprSources,
        E::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
        E: drizzle_core::expr::Expr<'a, MySQLValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        UpdateBuilder::from_sql(self.sql.append(crate::helpers::r#where(condition)))
    }
}

mutation_builder_methods!(
    UpdateBuilder,
    prepare: [UpdateWhereSet, UpdateOrderSet, UpdateLimitSet],
    order_by: [UpdateWhereSet] => UpdateOrderSet,
    limit: [UpdateWhereSet, UpdateOrderSet] => UpdateLimitSet,
);
