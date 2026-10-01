use crate::{common::MySQLSchemaType, helpers, values::MySQLValue};
use drizzle_core::{SQL, SQLTable, ToSQL, Token};

use super::ExecutableState;

pub use drizzle_core::builder::{
    SelectFromSet, SelectGroupSet, SelectInitial, SelectJoinSet, SelectLimitSet, SelectOffsetSet,
    SelectOrderSet, SelectSetOpSet, SelectWhereSet,
};

/// Builder state after `having`. A SELECT takes at most one HAVING.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectHavingSet;

impl ExecutableState for SelectHavingSet {}

/// Builder state after an index hint on the FROM table.
///
/// The FROM table takes one hint; `Kind` records which one.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectIndexHintSet<Kind>(core::marker::PhantomData<Kind>);

impl<Kind> ExecutableState for SelectIndexHintSet<Kind> {}

/// Marker for the MySQL `FOR UPDATE` lock strength.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ForUpdate;

/// Marker for the MySQL `FOR SHARE` lock strength.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ForShare;

/// Marker for a locking read without a wait modifier.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Wait;

/// Marker for a locking read with `NOWAIT`.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NoWait;

/// Marker for a locking read with `SKIP LOCKED`.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SkipLocked;

/// Builder state after `for_update` or `for_share`.
///
/// `Strength` is [`ForUpdate`] or [`ForShare`]; `Modifier` is [`Wait`],
/// [`NoWait`] or [`SkipLocked`]. Only `nowait` or `skip_locked` can follow,
/// once.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectForSet<Strength, Modifier = Wait>(core::marker::PhantomData<(Strength, Modifier)>);

impl<Strength, Modifier> ExecutableState for SelectForSet<Strength, Modifier> {}

/// Clause gate for SELECT methods whose names collide with INSERT/UPDATE/DELETE
/// builder methods on the shared `QueryBuilder` type.
///
/// Coherence can only rule out overlapping inherent impls through a trait
/// local to this crate, so these clauses use this trait instead of
/// [`drizzle_core::ClauseAllowed`].
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "builder state `{Self}` does not allow `{C}`",
    label = "not available at this point of the query",
    note = "SELECT clauses go in order: FROM, JOIN, WHERE, GROUP BY, HAVING, ORDER BY, LIMIT, OFFSET",
    note = "only a SELECT can be a set operand, a subquery, a derived table, or an INSERT source"
)]
pub trait SelectClause<C> {}

/// Clause marker for `OFFSET`, which is rendered with a `LIMIT` when none was
/// given.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct MySqlOffset;

// The shared SELECT states cover JOIN, GROUP BY, HAVING and the
// CTE/locking-read gate; MySQL adds its own states to those and keeps
// its own lists for clauses whose order rules differ.
impl SelectClause<drizzle_core::clause::Where> for SelectFromSet {}
impl<Kind> SelectClause<drizzle_core::clause::Where> for SelectIndexHintSet<Kind> {}
impl SelectClause<drizzle_core::clause::Where> for SelectJoinSet {}

impl SelectClause<drizzle_core::clause::OrderBy> for SelectFromSet {}
impl<Kind> SelectClause<drizzle_core::clause::OrderBy> for SelectIndexHintSet<Kind> {}
impl SelectClause<drizzle_core::clause::OrderBy> for SelectJoinSet {}
impl SelectClause<drizzle_core::clause::OrderBy> for SelectWhereSet {}
impl SelectClause<drizzle_core::clause::OrderBy> for SelectGroupSet {}
impl SelectClause<drizzle_core::clause::OrderBy> for SelectHavingSet {}

impl SelectClause<drizzle_core::clause::Limit> for SelectFromSet {}
impl<Kind> SelectClause<drizzle_core::clause::Limit> for SelectIndexHintSet<Kind> {}
impl SelectClause<drizzle_core::clause::Limit> for SelectJoinSet {}
impl SelectClause<drizzle_core::clause::Limit> for SelectWhereSet {}
impl SelectClause<drizzle_core::clause::Limit> for SelectGroupSet {}
impl SelectClause<drizzle_core::clause::Limit> for SelectHavingSet {}
impl SelectClause<drizzle_core::clause::Limit> for SelectOrderSet {}
impl SelectClause<drizzle_core::clause::Limit> for SelectSetOpSet {}

impl drizzle_core::ClauseAllowed<MySqlOffset> for SelectFromSet {}
impl<Kind> drizzle_core::ClauseAllowed<MySqlOffset> for SelectIndexHintSet<Kind> {}
impl drizzle_core::ClauseAllowed<MySqlOffset> for SelectJoinSet {}
impl drizzle_core::ClauseAllowed<MySqlOffset> for SelectWhereSet {}
impl drizzle_core::ClauseAllowed<MySqlOffset> for SelectGroupSet {}
impl drizzle_core::ClauseAllowed<MySqlOffset> for SelectHavingSet {}
impl drizzle_core::ClauseAllowed<MySqlOffset> for SelectOrderSet {}
impl drizzle_core::ClauseAllowed<MySqlOffset> for SelectSetOpSet {}

impl<Kind> drizzle_core::ClauseAllowed<drizzle_core::clause::Compound>
    for SelectIndexHintSet<Kind>
{
}
impl drizzle_core::ClauseAllowed<drizzle_core::clause::Compound> for SelectHavingSet {}

impl<Kind> drizzle_core::ClauseAllowed<drizzle_core::clause::Source> for SelectIndexHintSet<Kind> {}
impl drizzle_core::ClauseAllowed<drizzle_core::clause::Source> for SelectHavingSet {}
impl<Strength, Modifier> drizzle_core::ClauseAllowed<drizzle_core::clause::Source>
    for SelectForSet<Strength, Modifier>
{
}

impl<Kind> drizzle_core::ClauseAllowed<drizzle_core::clause::Join> for SelectIndexHintSet<Kind> {}

impl<Kind> drizzle_core::ClauseAllowed<drizzle_core::clause::GroupBy> for SelectIndexHintSet<Kind> {}

impl<Kind> drizzle_core::ClauseAllowed<drizzle_core::clause::Simple> for SelectIndexHintSet<Kind> {}
impl drizzle_core::ClauseAllowed<drizzle_core::clause::Simple> for SelectHavingSet {}

/// A `SELECT` query being built for `MySQL`.
///
/// This is [`QueryBuilder`](super::QueryBuilder) in one of the `Select*`
/// states. Start it with [`QueryBuilder::select`](super::QueryBuilder::select),
/// then call [`from`](Self::from).
///
/// # Clause order
///
/// Clauses must be added in SQL order. Each method is only available in the
/// states listed here:
///
/// | After | You can call |
/// |---|---|
/// | `select` | `from` |
/// | `from` | an index hint, joins, `where`, `group_by`, `order_by`, `limit`, `offset` |
/// | an index hint | joins, `where`, `group_by`, `order_by`, `limit`, `offset` |
/// | a join | more joins, `where`, `group_by`, `order_by`, `limit`, `offset` |
/// | `where` | `group_by`, `order_by`, `limit`, `offset` |
/// | `group_by` | `having`, `order_by`, `limit`, `offset` |
/// | `having` | `order_by`, `limit`, `offset` |
/// | `order_by` | `limit`, `offset` |
/// | `limit` | `offset` |
/// | a set operation | more set operations, `order_by`, `limit`, `offset` |
/// | `for_update` / `for_share` | `nowait` or `skip_locked` |
///
/// Every state from `from` on (except after a locking clause) also accepts
/// set operations (`union`, `intersect`, `except` and their `_all` forms).
/// Every state from `from` on, except a compound query, accepts a locking
/// clause (`for_update`, `for_share`) and [`into_cte`](Self::into_cte).
/// A finished query can be used as a subquery, named as a derived table
/// with [`alias`](Self::alias), or compiled with [`prepare`](Self::prepare).
///
/// # Compile-time checks
///
/// Besides clause order, the builder rejects: a second index hint on the
/// FROM table, a second HAVING, a set operation after a locking clause, a
/// locking clause on a compound query, and a WHERE, HAVING or JOIN
/// condition that is not boolean. Column references and grouping are
/// checked by [`prepare`](Self::prepare) and when the query runs.
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
///     .select((users.id, count(posts.id)))
///     .from(users)
///     .inner_join((posts, eq(posts.user_id, users.id)))
///     .r#where(eq(users.active, true))
///     .group_by(users.id)
///     .having(gt(count(posts.id), 0))
///     .order_by(desc(users.id))
///     .limit(10)
///     .offset(20);
/// // SELECT `users`.`id`, COUNT(`posts`.`id`) FROM `users`
/// //   INNER JOIN `posts` ON `posts`.`user_id` = `users`.`id`
/// //   WHERE `users`.`active` = ? GROUP BY `users`.`id`
/// //   HAVING COUNT(`posts`.`id`)> ? ORDER BY `users`.`id` DESC LIMIT ? OFFSET ?
/// # "####;
/// ```
pub type SelectBuilder<'a, Schema, State, Table = (), Marker = (), Row = (), Grouped = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row, Grouped>;

mod private {
    use super::{
        SelectForSet, SelectFromSet, SelectGroupSet, SelectHavingSet, SelectIndexHintSet,
        SelectJoinSet, SelectLimitSet, SelectOffsetSet, SelectOrderSet, SelectSetOpSet,
        SelectWhereSet,
    };

    pub trait SealedSelect {}

    pub trait Prepare {}

    impl Prepare for SelectFromSet {}
    impl<Kind> Prepare for SelectIndexHintSet<Kind> {}
    impl Prepare for SelectJoinSet {}
    impl Prepare for SelectWhereSet {}
    impl Prepare for SelectGroupSet {}
    impl Prepare for SelectHavingSet {}
    impl Prepare for SelectOrderSet {}
    impl Prepare for SelectLimitSet {}
    impl Prepare for SelectOffsetSet {}
    impl Prepare for SelectSetOpSet {}
    impl<Strength, Modifier> Prepare for SelectForSet<Strength, Modifier> {}
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: private::Prepare,
{
    /// Renders this query as a [`PreparedStatement`](drizzle_core::prepared::PreparedStatement).
    ///
    /// `MySQL` binds parameters by position, so named placeholders are put in
    /// the order they appear in the SQL (a name used twice is bound twice).
    /// Bind values by name later with `bind`.
    ///
    /// # Compile-time checks
    ///
    /// Only compiles when every column reference is in scope (its table is
    /// in FROM or a JOIN) and every selected column is grouped or inside an
    /// aggregate.
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
    ///     .select(users.id)
    ///     .from(users)
    ///     .r#where(eq(users.name, Placeholder::named("name")));
    /// let prepared = query.prepare();
    /// assert_eq!(prepared.sql(), "SELECT `users`.`id` FROM `users` WHERE `users`.`name` = ?");
    /// # "####;
    /// ```
    #[must_use]
    pub fn prepare<ScopeProof, AggProof>(
        &self,
    ) -> drizzle_core::prepared::PreparedStatement<'a, MySQLValue<'a>>
    where
        M: drizzle_core::row::MarkerScopeValidFor<ScopeProof>
            + drizzle_core::row::MarkerAggValidFor<G, AggProof>,
    {
        self.prepared_statement()
    }
}

impl<'a, S, M> SelectBuilder<'a, S, SelectInitial, (), M> {
    /// Sets the FROM source: a table, a CTE, or a derived table made with
    /// [`alias`](Self::alias).
    ///
    /// The result row type is inferred from the selected columns and this
    /// source. With `select(())`, the row is the table's generated select
    /// model.
    #[allow(clippy::type_complexity)]
    pub fn from<T>(
        self,
        table: T,
    ) -> SelectBuilder<
        'a,
        S,
        SelectFromSet,
        T,
        drizzle_core::FromMarker<M, T>,
        <M as drizzle_core::ResolveRow<T>>::Row,
    >
    where
        T: ToSQL<'a, MySQLValue<'a>> + drizzle_core::ScopeEntry,
        M: drizzle_core::ResolveRow<T>,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::from(table)))
    }
}

impl<'a, S, T, M, R, G> SelectBuilder<'a, S, SelectFromSet, T, M, R, G>
where
    T: crate::traits::MySQLTable<'a>,
{
    /// Adds `USE INDEX (..)` to the FROM table.
    ///
    /// Pass one `#[MySQLIndex]` index of this table, or a tuple of up to
    /// eight. An index of another table does not compile. Only one hint
    /// (`use_index`, `force_index` or `ignore_index`) can be added, right
    /// after `from`. To hint a joined table, use [`MySQLIndexHintExt`](crate::helpers::MySQLIndexHintExt).
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
    ///     .select(users.id)
    ///     .from(users)
    ///     .use_index(UsersNameIdx::new());
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     "SELECT `users`.`id` FROM `users` USE INDEX (`users_name_idx`)"
    /// );
    /// # "####;
    /// ```
    #[must_use]
    pub fn use_index<Indexes>(
        self,
        indexes: Indexes,
    ) -> SelectBuilder<'a, S, SelectIndexHintSet<helpers::UseIndex>, T, M, R, G>
    where
        Indexes: helpers::IndexHintList<'a, T>,
    {
        SelectBuilder::from_sql(self.sql.append(
            helpers::index_hint::<T, Indexes, helpers::UseIndex>(&indexes),
        ))
    }

    /// Adds `FORCE INDEX (..)` to the FROM table, which tells `MySQL` to use
    /// a table scan only if none of these indexes can be used. See
    /// [`use_index`](Self::use_index).
    #[must_use]
    pub fn force_index<Indexes>(
        self,
        indexes: Indexes,
    ) -> SelectBuilder<'a, S, SelectIndexHintSet<helpers::ForceIndex>, T, M, R, G>
    where
        Indexes: helpers::IndexHintList<'a, T>,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::index_hint::<
            T,
            Indexes,
            helpers::ForceIndex,
        >(&indexes)))
    }

    /// Adds `IGNORE INDEX (..)` to the FROM table. See
    /// [`use_index`](Self::use_index).
    #[must_use]
    pub fn ignore_index<Indexes>(
        self,
        indexes: Indexes,
    ) -> SelectBuilder<'a, S, SelectIndexHintSet<helpers::IgnoreIndex>, T, M, R, G>
    where
        Indexes: helpers::IndexHintList<'a, T>,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::index_hint::<
            T,
            Indexes,
            helpers::IgnoreIndex,
        >(&indexes)))
    }
}

macro_rules! join_on_method {
    ($name:ident, $join:expr, $kind:ident) => {
        #[doc = concat!("Adds a join with `", stringify!($name), "`.")]
        ///
        /// Pass `(source, condition)` with a boolean condition, or a bare table
        /// to join on its foreign key to the previous table. The source can be
        /// a table, a derived table (see [`alias`](Self::alias)), or a table
        /// with an index hint (see [`MySQLIndexHintExt`](crate::helpers::MySQLIndexHintExt)).
        /// With `select(())`, a LEFT or RIGHT join makes the side that may be
        /// missing an `Option` in the row type.
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
        ///     .select((users.name, posts.title))
        ///     .from(users)
        ///     .left_join((posts, eq(posts.user_id, users.id)));
        /// assert_eq!(
        ///     query.to_sql().sql(),
        ///     "SELECT `users`.`name`, `posts`.`title` FROM `users` LEFT JOIN `posts` ON `posts`.`user_id` = `users`.`id`"
        /// );
        /// # "####;
        /// ```
        #[allow(clippy::type_complexity)]
        pub fn $name<J: helpers::JoinArg<'a, T>>(
            self,
            arg: J,
        ) -> SelectBuilder<
            'a,
            S,
            SelectJoinSet,
            J::JoinedTable,
            <M as drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::$kind, J::OnSources>>::Marker,
            <M as drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::$kind, J::OnSources>>::Row,
            G,
        >
        where
            M: drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::$kind, J::OnSources>,
        {
            SelectBuilder::from_sql(self.sql.append(arg.into_join_sql($join)))
        }
    };
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Join>,
{
    join_on_method!(join, drizzle_core::Join::new(), InnerJoin);
    join_on_method!(inner_join, drizzle_core::Join::new().inner(), InnerJoin);
    join_on_method!(left_join, drizzle_core::Join::new().left(), LeftJoin);
    join_on_method!(
        left_outer_join,
        drizzle_core::Join::new().left().outer(),
        LeftJoin
    );
    join_on_method!(right_join, drizzle_core::Join::new().right(), RightJoin);
    join_on_method!(
        right_outer_join,
        drizzle_core::Join::new().right().outer(),
        RightJoin
    );

    /// Adds a `CROSS JOIN`, which pairs every row with every row of `arg`.
    ///
    /// A bare source renders `CROSS JOIN`. For backwards compatibility,
    /// `(source, condition)` renders the equivalent `INNER JOIN ... ON ...`.
    #[allow(clippy::type_complexity)]
    pub fn cross_join<Arg: helpers::CrossJoinArg<'a, T>>(
        self,
        arg: Arg,
    ) -> SelectBuilder<
        'a,
        S,
        SelectJoinSet,
        Arg::JoinedTable,
        <M as drizzle_core::JoinStep<
            R,
            Arg::JoinedTable,
            drizzle_core::InnerJoin,
            Arg::OnSources,
        >>::Marker,
        <M as drizzle_core::JoinStep<
            R,
            Arg::JoinedTable,
            drizzle_core::InnerJoin,
            Arg::OnSources,
        >>::Row,
        G,
    >
    where
        M: drizzle_core::JoinStep<R, Arg::JoinedTable, drizzle_core::InnerJoin, Arg::OnSources>,
    {
        SelectBuilder::from_sql(self.sql.append(arg.into_cross_join_sql()))
    }

    /// Adds `INNER JOIN LATERAL (subquery) AS name ON condition`.
    ///
    /// A lateral subquery may reference columns of the tables before it.
    /// Requires `MySQL` 8.0.14 or later.
    #[allow(clippy::type_complexity)]
    pub fn inner_join_lateral<Arg>(
        self,
        arg: Arg,
    ) -> SelectBuilder<
        'a,
        S,
        SelectJoinSet,
        Arg::JoinedTable,
        <M as drizzle_core::JoinStep<
            R,
            Arg::JoinedTable,
            drizzle_core::Lateral<drizzle_core::InnerJoin>,
            Arg::OnSources,
        >>::Marker,
        <M as drizzle_core::JoinStep<
            R,
            Arg::JoinedTable,
            drizzle_core::Lateral<drizzle_core::InnerJoin>,
            Arg::OnSources,
        >>::Row,
        G,
    >
    where
        Arg: drizzle_core::LateralArg<'a, MySQLValue<'a>>,
        M: drizzle_core::JoinStep<
                R,
                Arg::JoinedTable,
                drizzle_core::Lateral<drizzle_core::InnerJoin>,
                Arg::OnSources,
            >,
    {
        SelectBuilder::from_sql(
            self.sql
                .append(arg.into_lateral_sql(drizzle_core::Join::new().inner())),
        )
    }

    /// Adds `LEFT JOIN LATERAL (subquery) AS name ON condition`.
    ///
    /// Like [`inner_join_lateral`](Self::inner_join_lateral), but keeps rows
    /// with no match. The selection must allow the lateral columns to be
    /// missing.
    #[allow(clippy::type_complexity)]
    pub fn left_join_lateral<Arg, SelectionProof>(
        self,
        arg: Arg,
    ) -> SelectBuilder<
        'a,
        S,
        SelectJoinSet,
        Arg::JoinedTable,
        <M as drizzle_core::JoinStep<
            R,
            Arg::JoinedTable,
            drizzle_core::Lateral<drizzle_core::LeftJoin>,
            Arg::OnSources,
        >>::Marker,
        <M as drizzle_core::JoinStep<
            R,
            Arg::JoinedTable,
            drizzle_core::Lateral<drizzle_core::LeftJoin>,
            Arg::OnSources,
        >>::Row,
        G,
    >
    where
        Arg: drizzle_core::LateralArg<'a, MySQLValue<'a>>,
        M: drizzle_core::JoinStep<
                R,
                Arg::JoinedTable,
                drizzle_core::Lateral<drizzle_core::LeftJoin>,
                Arg::OnSources,
            > + drizzle_core::LeftLateralSelection<SelectionProof>,
    {
        SelectBuilder::from_sql(
            self.sql
                .append(arg.into_lateral_sql(drizzle_core::Join::new().left())),
        )
    }

    /// Adds `CROSS JOIN LATERAL (subquery) AS name`, with no ON condition.
    #[allow(clippy::type_complexity)]
    pub fn cross_join_lateral<Source>(
        self,
        source: Source,
    ) -> SelectBuilder<
        'a,
        S,
        SelectJoinSet,
        Source::JoinedTable,
        <M as drizzle_core::JoinStep<
            R,
            Source::JoinedTable,
            drizzle_core::Lateral<drizzle_core::InnerJoin>,
        >>::Marker,
        <M as drizzle_core::JoinStep<
            R,
            Source::JoinedTable,
            drizzle_core::Lateral<drizzle_core::InnerJoin>,
        >>::Row,
        G,
    >
    where
        Source: drizzle_core::LateralSource<'a, MySQLValue<'a>>,
        M: drizzle_core::JoinStep<
                R,
                Source::JoinedTable,
                drizzle_core::Lateral<drizzle_core::InnerJoin>,
            >,
    {
        SelectBuilder::from_sql(self.sql.append(source.into_cross_lateral_sql()))
    }
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: SelectClause<drizzle_core::clause::Where>,
{
    /// Adds a WHERE clause. The condition must be a boolean expression.
    #[allow(clippy::type_complexity)]
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> SelectBuilder<
        'a,
        S,
        SelectWhereSet,
        T,
        <M as drizzle_core::HasScope>::With<E::Sources>,
        R,
        G,
    >
    where
        M: drizzle_core::HasScope,
        E: drizzle_core::expr::Expr<'a, MySQLValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::r#where(condition)))
    }
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::GroupBy>,
{
    /// Adds a GROUP BY clause. Pass one expression or a tuple.
    ///
    /// Every selected column that is not inside an aggregate must appear in
    /// the GROUP BY list, or belong to a table grouped by its single-column
    /// primary key. This is checked by `prepare` and when the query runs.
    #[allow(clippy::type_complexity)]
    pub fn group_by<Gr>(
        self,
        columns: Gr,
    ) -> SelectBuilder<
        'a,
        S,
        SelectGroupSet,
        T,
        <M as drizzle_core::HasScope>::With<Gr::Sources>,
        R,
        Gr::Columns,
    >
    where
        M: drizzle_core::HasScope,
        Gr: drizzle_core::IntoGroupBy<'a, MySQLValue<'a>>,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::group_by_expr(columns)))
    }
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Having>,
{
    /// Adds a HAVING clause, which filters groups.
    ///
    /// Only available after `group_by`, and only once. The condition must
    /// be a boolean expression and may use aggregates.
    #[allow(clippy::type_complexity)]
    pub fn having<E>(
        self,
        condition: E,
    ) -> SelectBuilder<
        'a,
        S,
        SelectHavingSet,
        T,
        <M as drizzle_core::HasScope>::With<E::Sources>,
        R,
        G,
    >
    where
        M: drizzle_core::HasScope,
        E: drizzle_core::expr::Expr<'a, MySQLValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::having(condition)))
    }
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: SelectClause<drizzle_core::clause::OrderBy>,
{
    /// Adds an ORDER BY clause. Pass one ordering term or a tuple; wrap a
    /// column in [`asc`](crate::helpers::asc) or
    /// [`desc`](crate::helpers::desc) to set the direction.
    #[allow(clippy::type_complexity)]
    pub fn order_by<O>(
        self,
        order: O,
    ) -> SelectBuilder<
        'a,
        S,
        SelectOrderSet,
        T,
        <M as drizzle_core::HasScope>::With<O::Sources>,
        R,
        G,
    >
    where
        M: drizzle_core::HasScope,
        O: ToSQL<'a, MySQLValue<'a>> + drizzle_core::expr::ExprSources,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::order_by(order)))
    }
}

impl<'a, S, T, M, R, G> SelectBuilder<'a, S, SelectSetOpSet, T, M, R, G> {
    /// Sorts a compound (set operation) result.
    ///
    /// Pass a selected column (rendered without its table name), or
    /// [`output_alias`](crate::helpers::output_alias) for a named output,
    /// optionally wrapped in [`asc`](crate::helpers::asc) or
    /// [`desc`](crate::helpers::desc). A column the first query does not
    /// select does not compile.
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
    ///     .select(alias(users.name, "label"))
    ///     .from(users)
    ///     .union(builder.select(alias(posts.title, "label")).from(posts))
    ///     .order_by(desc(output_alias("label")));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     "(SELECT `users`.`name` AS `label` FROM `users`) UNION (SELECT `posts`.`title` AS `label` FROM `posts`) ORDER BY `label` DESC"
    /// );
    /// # "####;
    /// ```
    pub fn order_by<O, Proof>(self, order: O) -> SelectBuilder<'a, S, SelectOrderSet, T, M, R, G>
    where
        O: helpers::SetOrderBy<'a, M, T, Proof>,
    {
        let order = helpers::SetOrderBy::into_set_order_sql(order);
        SelectBuilder::from_sql(
            self.sql
                .append(SQL::from_iter([Token::ORDER, Token::BY]).append(order)),
        )
    }
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: SelectClause<drizzle_core::clause::Limit>,
{
    /// Adds a LIMIT clause.
    ///
    /// Pass a non-negative integer or an integer placeholder. Both are sent
    /// as bound parameters.
    ///
    /// # Panics
    ///
    /// Panics when a numeric argument is negative or does not fit in
    /// `usize`.
    #[track_caller]
    pub fn limit<P>(self, limit: P) -> SelectBuilder<'a, S, SelectLimitSet, T, M, R, G>
    where
        P: drizzle_core::PaginationArg<'a, MySQLValue<'a>>,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::limit(limit)))
    }
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<MySqlOffset>,
{
    /// Skips the first `offset` rows without limiting the row count.
    ///
    /// `MySQL` has no bare `OFFSET`, so this renders
    /// `LIMIT 9223372036854775807 OFFSET ?` (see
    /// [`drizzle_core::helpers::MYSQL_UNBOUNDED_LIMIT`]).
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
    /// let query = builder.select(users.id).from(users).offset(5);
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     "SELECT `users`.`id` FROM `users` LIMIT 9223372036854775807 OFFSET ?"
    /// );
    /// # "####;
    /// ```
    ///
    /// # Panics
    ///
    /// Panics when a numeric argument is negative or does not fit in
    /// `usize`.
    #[track_caller]
    pub fn offset<P>(self, offset: P) -> SelectBuilder<'a, S, SelectOffsetSet, T, M, R, G>
    where
        P: drizzle_core::PaginationArg<'a, MySQLValue<'a>>,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::standalone_offset(offset)))
    }
}

impl<'a, S, T, M, R, G> SelectBuilder<'a, S, SelectLimitSet, T, M, R, G> {
    /// Adds an OFFSET clause after LIMIT.
    ///
    /// # Panics
    ///
    /// Panics when a numeric argument is negative or does not fit in
    /// `usize`.
    #[track_caller]
    pub fn offset<P>(self, offset: P) -> SelectBuilder<'a, S, SelectOffsetSet, T, M, R, G>
    where
        P: drizzle_core::PaginationArg<'a, MySQLValue<'a>>,
    {
        SelectBuilder::from_sql(self.sql.append(drizzle_core::helpers::offset(offset)))
    }
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Simple>,
{
    /// Adds `FOR UPDATE`, which locks the selected rows until the
    /// transaction ends.
    ///
    /// Must be the last clause; only [`nowait`](Self::nowait) or
    /// [`skip_locked`](Self::skip_locked) can follow. Not available on a
    /// compound query, and the result cannot be a set operand.
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
    ///     .select(users.id)
    ///     .from(users)
    ///     .limit(2)
    ///     .for_update()
    ///     .skip_locked();
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     "SELECT `users`.`id` FROM `users` LIMIT ? FOR UPDATE SKIP LOCKED"
    /// );
    /// # "####;
    /// ```
    #[must_use]
    pub fn for_update(self) -> SelectBuilder<'a, S, SelectForSet<ForUpdate>, T, M, R, G> {
        SelectBuilder::from_sql(
            self.sql
                .push(drizzle_core::Token::FOR)
                .push(drizzle_core::Token::UPDATE),
        )
    }

    /// Adds `FOR SHARE`, which takes shared locks on the selected rows: other
    /// transactions can read them but not change them. See
    /// [`for_update`](Self::for_update).
    #[must_use]
    pub fn for_share(self) -> SelectBuilder<'a, S, SelectForSet<ForShare>, T, M, R, G> {
        SelectBuilder::from_sql(
            self.sql
                .push(drizzle_core::Token::FOR)
                .push(drizzle_core::Token::SHARE),
        )
    }
}

impl<'a, S, Strength, T, M, R, G> SelectBuilder<'a, S, SelectForSet<Strength, Wait>, T, M, R, G> {
    /// Adds `NOWAIT`: fail at once instead of waiting when a row is locked
    /// by another transaction.
    #[must_use]
    pub fn nowait(self) -> SelectBuilder<'a, S, SelectForSet<Strength, NoWait>, T, M, R, G> {
        SelectBuilder::from_sql(self.sql.push(drizzle_core::Token::NOWAIT))
    }

    /// Adds `SKIP LOCKED`: leave out rows locked by another transaction
    /// instead of waiting for them.
    #[must_use]
    pub fn skip_locked(
        self,
    ) -> SelectBuilder<'a, S, SelectForSet<Strength, SkipLocked>, T, M, R, G> {
        SelectBuilder::from_sql(
            self.sql
                .push(drizzle_core::Token::SKIP)
                .push(drizzle_core::Token::LOCKED),
        )
    }
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Simple> + ExecutableState,
    T: SQLTable<'a, MySQLSchemaType, MySQLValue<'a>>,
{
    /// Turns this SELECT into a common table expression named `Tag::NAME`.
    ///
    /// The result derefs to an aliased copy of the FROM table, so you can
    /// select its columns with the usual field access. Pass it to
    /// [`QueryBuilder::with`](super::QueryBuilder::with), which has an
    /// example. Not available on a compound query.
    #[must_use]
    pub fn into_cte<Tag: drizzle_core::Tag + 'static>(
        self,
    ) -> super::CTEView<'a, <T as SQLTable<'a, MySQLSchemaType, MySQLValue<'a>>>::Aliased<Tag>, Self>
    {
        super::CTEView::new(
            <T as SQLTable<'a, MySQLSchemaType, MySQLValue<'a>>>::alias::<Tag>(),
            Tag::NAME,
            self,
        )
    }
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Source>,
{
    /// Names this query so it can be used as a derived table in `from` or a
    /// join.
    ///
    /// `tag` is a value of a [`Tag`](drizzle_core::Tag) type; its `NAME`
    /// becomes the SQL alias. The result exposes the selected columns, so
    /// the outer query can reference them with typed accessors.
    ///
    /// # Panics
    ///
    /// Panics when the projection contains duplicate output names. Name a
    /// computed expression with [`drizzle_core::expr::AliasExt::named`] to
    /// make each output unique.
    #[must_use]
    pub fn alias<Tag, AggProof>(
        self,
        _tag: Tag,
    ) -> drizzle_core::Derived<
        'a,
        MySQLValue<'a>,
        Tag,
        <M as drizzle_core::DerivedSelection<'a, MySQLValue<'a>, MySQLSchemaType, T>>::Projection,
        Self,
    >
    where
        Tag: drizzle_core::Tag,
        M: drizzle_core::DerivedSelection<'a, MySQLValue<'a>, MySQLSchemaType, T>
            + drizzle_core::row::MarkerAggValidFor<G, AggProof>,
        <M as drizzle_core::DerivedSelection<'a, MySQLValue<'a>, MySQLSchemaType, T>>::Projection:
            drizzle_core::DerivedProjection<Tag>,
    {
        // SAFETY: The executable-state, aggregate, and projection bounds
        // above prove that this query matches the derived projection; its
        // scope travels in `Self`'s sources and is checked where it is used.
        unsafe { drizzle_core::Derived::new_unchecked(self) }
    }
}

macro_rules! set_operation {
    ($name:ident, $token:expr, $all:expr) => {
        #[doc = concat!("Combines this query with `other` using `", stringify!($name), "`.")]
        ///
        /// Both queries must select the same row type. Each operand is wrapped
        /// in parentheses, so its own ORDER BY and LIMIT stay with it. After a
        /// set operation you can chain more set operations, then `order_by`,
        /// `limit` and `offset` for the combined result. `INTERSECT` and
        /// `EXCEPT` need `MySQL` 8.0.31 or later.
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
        ///     .select(users.id)
        ///     .from(users)
        ///     .union(builder.select(posts.user_id).from(posts));
        /// assert_eq!(
        ///     query.to_sql().sql(),
        ///     "(SELECT `users`.`id` FROM `users`) UNION (SELECT `posts`.`user_id` FROM `posts`)"
        /// );
        /// # "####;
        /// ```
        #[allow(clippy::type_complexity)]
        pub fn $name<O>(
            self,
            other: O,
        ) -> SelectBuilder<
            'a,
            S,
            SelectSetOpSet,
            T,
            <M as drizzle_core::SetOperand<O::Marker>>::Combined,
            R,
            G,
        >
        where
            O: IntoSelectQuery<'a, S, R>,
            M: drizzle_core::SetOperand<O::Marker>,
        {
            SelectBuilder::from_sql(helpers::set_op(
                self.sql,
                $token,
                $all,
                other.into_select_query().into_select_sql(),
            ))
        }
    };
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Compound>,
{
    set_operation!(union, drizzle_core::Token::UNION, false);
    set_operation!(union_all, drizzle_core::Token::UNION, true);
    set_operation!(intersect, drizzle_core::Token::INTERSECT, false);
    set_operation!(intersect_all, drizzle_core::Token::INTERSECT, true);
    set_operation!(except, drizzle_core::Token::EXCEPT, false);
    set_operation!(except_all, drizzle_core::Token::EXCEPT, true);
}

/// A finished SELECT with row type `R`.
///
/// This trait is sealed so INSERT ... SELECT and set operations cannot accept
/// arbitrary SQL or DML builders.
#[doc(hidden)]
pub trait CompletedSelect<'a, S, R>: private::SealedSelect {
    type Marker;
    type Grouped;

    fn into_select_sql(self) -> drizzle_core::SQL<'a, MySQLValue<'a>>;
}

/// Converts a finished SELECT, or a driver builder that wraps one, into its
/// [`CompletedSelect`].
///
/// Implementations must unwrap to the sealed [`CompletedSelect`] type; they cannot
/// manufacture an arbitrary SQL fragment or row marker.
#[doc(hidden)]
pub trait IntoSelectQuery<'a, S, R> {
    type Marker;
    type Grouped;
    type Select: CompletedSelect<'a, S, R, Marker = Self::Marker, Grouped = Self::Grouped>;

    fn into_select_query(self) -> Self::Select;
}

impl<'a, S, State, T, M, R, G> private::SealedSelect for SelectBuilder<'a, S, State, T, M, R, G> where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Compound>
{
}

impl<'a, S, State, T, M, R, G> CompletedSelect<'a, S, R> for SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Compound>,
{
    type Marker = M;
    type Grouped = G;

    fn into_select_sql(self) -> drizzle_core::SQL<'a, MySQLValue<'a>> {
        self.sql
    }
}

impl<'a, S, State, T, M, R, G> IntoSelectQuery<'a, S, R> for SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Compound>,
{
    type Marker = M;
    type Grouped = G;
    type Select = Self;

    fn into_select_query(self) -> Self::Select {
        self
    }
}

impl<'a, S, State, T, M, R, G> drizzle_core::expr::Expr<'a, MySQLValue<'a>>
    for SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Compound> + ExecutableState,
    M: drizzle_core::expr::SubqueryType<'a, MySQLValue<'a>> + drizzle_core::SelectSources,
{
    type SQLType = <M as drizzle_core::expr::SubqueryType<'a, MySQLValue<'a>>>::SQLType;
    type Nullable = drizzle_core::expr::Null;
    type Aggregate = drizzle_core::expr::Scalar;
}

impl<S, State: drizzle_core::ClauseAllowed<drizzle_core::clause::Source>, T, M, R, G>
    drizzle_core::expr::SelectQuery for SelectBuilder<'_, S, State, T, M, R, G>
{
}

impl<S, State, T, M, R, G> drizzle_core::expr::ExprSources
    for SelectBuilder<'_, S, State, T, M, R, G>
where
    M: drizzle_core::SelectSources,
{
    type Sources = M::Sources;
}
