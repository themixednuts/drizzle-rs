use super::select::{CompletedSelect, IntoSelectQuery};
use crate::{traits::MySQLTable, values::MySQLValue};
use drizzle_core::{
    IncludesRequired, InsertSelectCompatible, InsertSelectTable, InsertTargetColumns,
    PartialInsertSelectCompatible, ToSQL,
};

pub use drizzle_core::builder::{InsertColumnsSet, InsertInitial, InsertValuesSet};

/// Builder state after [`ignore`](InsertBuilder::ignore), before the rows
/// are given.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertIgnoreSet;

/// Builder state after
/// [`on_duplicate_key_update`](InsertBuilder::on_duplicate_key_update).
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertOnDuplicateKeyUpdateSet;

impl drizzle_core::ExecutableState for InsertOnDuplicateKeyUpdateSet {}

/// An `INSERT` query being built for `MySQL`.
///
/// This is [`QueryBuilder`](super::QueryBuilder) in one of the `Insert*`
/// states. Start it with [`QueryBuilder::insert`](super::QueryBuilder::insert).
///
/// # Clause order
///
/// 1. Optionally [`ignore`](Self::ignore), for `INSERT IGNORE`.
/// 2. A row source: [`values`](Self::values) or [`value`](Self::value),
///    [`select`](Self::select), or [`columns`](Self::columns) followed by
///    `select`.
/// 3. Optionally [`on_duplicate_key_update`](Self::on_duplicate_key_update).
///
/// `MySQL` has no `RETURNING`; the driver reports the affected row count
/// and last insert id instead. [`prepare`](Self::prepare) is available once
/// the rows are given.
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
///     .insert(users)
///     .value(InsertUsers::new("Alice"))
///     .on_duplicate_key_update(UpdateUsers::default().with_name("Alice"));
/// assert_eq!(
///     query.to_sql().sql(),
///     "INSERT INTO `users` (`name`) VALUES (?) ON DUPLICATE KEY UPDATE `name` = ?"
/// );
/// # "####;
/// ```
pub type InsertBuilder<'a, Schema, State, Table, Marker = (), Row = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row>;

impl<'a, S, T, M, R> InsertBuilder<'a, S, InsertValuesSet, T, M, R> {
    /// Renders this statement as a [`PreparedStatement`](drizzle_core::prepared::PreparedStatement)
    /// whose placeholders are bound in `MySQL`'s positional order.
    #[must_use]
    pub fn prepare(&self) -> drizzle_core::prepared::PreparedStatement<'a, MySQLValue<'a>> {
        self.prepared_statement()
    }

    /// Adds `ON DUPLICATE KEY UPDATE`, which updates the existing row when
    /// the new one would duplicate any primary or unique key.
    ///
    /// `values` is the table's generated update model. `MySQL` picks the
    /// conflicting key itself, so there is no conflict target.
    ///
    /// # Panics
    ///
    /// Panics when `values` sets no column.
    #[track_caller]
    pub fn on_duplicate_key_update(
        self,
        values: T::Update,
    ) -> InsertBuilder<'a, S, InsertOnDuplicateKeyUpdateSet, T, M, R>
    where
        T: MySQLTable<'a>,
    {
        let sql = crate::helpers::on_duplicate_key_update::<T>(&values);
        drop(values);
        InsertBuilder::from_sql(self.sql.append(sql))
    }
}

impl<'a, S, T, M, R> InsertBuilder<'a, S, InsertOnDuplicateKeyUpdateSet, T, M, R> {
    /// Renders this statement as a [`PreparedStatement`](drizzle_core::prepared::PreparedStatement)
    /// whose placeholders are bound in `MySQL`'s positional order.
    #[must_use]
    pub fn prepare(&self) -> drizzle_core::prepared::PreparedStatement<'a, MySQLValue<'a>> {
        self.prepared_statement()
    }
}

impl<'a, Schema, Table> InsertBuilder<'a, Schema, InsertInitial, Table>
where
    Table: MySQLTable<'a>,
{
    /// Turns this statement into `INSERT IGNORE`.
    ///
    /// `MySQL` then skips rows that duplicate a key, and also turns many other
    /// errors (such as out-of-range values) into warnings. Use it only when
    /// that is what you want; for duplicates alone, prefer
    /// [`on_duplicate_key_update`](Self::on_duplicate_key_update).
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
    /// let query = builder.insert(users).ignore().value(InsertUsers::new("Alice"));
    /// assert_eq!(query.to_sql().sql(), "INSERT IGNORE INTO `users` (`name`) VALUES (?)");
    /// # "####;
    /// ```
    #[must_use]
    pub fn ignore(self) -> InsertBuilder<'a, Schema, InsertIgnoreSet, Table> {
        InsertBuilder::from_sql(crate::helpers::insert_ignore(self.sql))
    }
}

/// Insert states that still accept a VALUES or SELECT row source.
#[doc(hidden)]
pub trait InsertRowSourceState {}

impl InsertRowSourceState for InsertInitial {}
impl InsertRowSourceState for InsertIgnoreSet {}

impl<'a, Schema, State, Table> InsertBuilder<'a, Schema, State, Table>
where
    State: InsertRowSourceState,
    Table: MySQLTable<'a>,
{
    /// Lists the target columns for an `INSERT ... SELECT`.
    ///
    /// Pass a tuple of the table's columns, then call
    /// [`select`](InsertBuilder::select). The list must include every
    /// required column (one without a default), and the SELECT must produce
    /// matching types in the same order; both are checked at compile time.
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
    ///     .insert(posts)
    ///     .columns((posts.user_id, posts.title))
    ///     .select(builder.select((users.id, users.name)).from(users));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     "INSERT INTO `posts` (`user_id`, `title`) SELECT `users`.`id`, `users`.`name` FROM `users`"
    /// );
    /// # "####;
    /// ```
    ///
    /// # Panics
    ///
    /// Panics when the same target column appears more than once.
    pub fn columns<Columns>(
        self,
        columns: Columns,
    ) -> InsertBuilder<'a, Schema, InsertColumnsSet<Columns::Columns>, Table>
    where
        Columns: InsertTargetColumns<'a, MySQLValue<'a>, Table>,
    {
        InsertBuilder::from_sql(self.sql.append(columns.into_target_columns_sql()))
    }

    /// Inserts one row. Same as `values([value])`.
    ///
    /// `value` is the table's generated insert model (for example
    /// `InsertUsers`).
    pub fn value<T>(
        self,
        value: Table::Insert<T>,
    ) -> InsertBuilder<'a, Schema, InsertValuesSet, Table> {
        self.values([value])
    }

    /// Inserts one or more rows.
    ///
    /// Each item is the table's generated insert model. Rows may set
    /// different columns: every row then lists all of them, with `DEFAULT`
    /// for the ones it leaves out.
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
    ///     .insert(users)
    ///     .values([InsertUsers::new("Alice"), InsertUsers::new("Bob")]);
    /// assert_eq!(query.to_sql().sql(), "INSERT INTO `users` (`name`) VALUES (?), (?)");
    /// # "####;
    /// ```
    ///
    /// # Panics
    ///
    /// Panics when `values` is empty.
    #[track_caller]
    pub fn values<I, T>(self, values: I) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        I: IntoIterator<Item = Table::Insert<T>>,
    {
        InsertBuilder::from_sql(
            self.sql
                .append(crate::helpers::values::<'a, Table, T>(values)),
        )
    }

    /// Inserts the rows of a SELECT into every insertable column of the
    /// table.
    ///
    /// The SELECT must produce one value per insertable column, in table
    /// order, with compatible types and nullability; its column references
    /// and aggregates are checked too. To fill only some columns, call
    /// [`columns`](Self::columns) first. A WITH clause belongs to the inner
    /// SELECT, so it renders after `INSERT INTO table (...)`.
    pub fn select<Q, R, ScopeProof, AggProof>(
        self,
        query: Q,
    ) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        Table: InsertSelectTable,
        Q: IntoSelectQuery<'a, Schema, R>,
        Q::Marker: InsertSelectCompatible<'a, MySQLValue<'a>, Table, R>
            + drizzle_core::MarkerScopeValidFor<ScopeProof>
            + drizzle_core::MarkerAggValidFor<Q::Grouped, AggProof>,
    {
        InsertBuilder::from_sql(
            self.sql
                .append(Table::insert_columns_sql::<MySQLValue<'a>>())
                .append(query.into_select_query().into_select_sql()),
        )
    }

    /// Appends any SQL as the row source, without a column list.
    ///
    /// Nothing is checked: not the column count, types, nullability, column
    /// scope or aggregates. Prefer [`select`](Self::select).
    pub fn select_raw<Q>(self, query: Q) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        Q: ToSQL<'a, MySQLValue<'a>>,
    {
        InsertBuilder::from_sql(self.sql.append(query.into_sql()))
    }
}

impl<'a, Schema, Table, Targets> InsertBuilder<'a, Schema, InsertColumnsSet<Targets>, Table>
where
    Table: MySQLTable<'a>,
{
    /// Inserts the rows of a SELECT into the columns chosen with
    /// [`columns`](InsertBuilder::columns).
    ///
    /// The SELECT must produce one value per chosen column, in the same
    /// order, with compatible types and nullability.
    pub fn select<Q, R, RequiredProof, ScopeProof, AggProof>(
        self,
        query: Q,
    ) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        Targets: IncludesRequired<Table::RequiredColumns, RequiredProof>,
        Table: InsertSelectTable,
        Q: IntoSelectQuery<'a, Schema, R>,
        Q::Marker: PartialInsertSelectCompatible<'a, MySQLValue<'a>, Targets>
            + drizzle_core::MarkerScopeValidFor<ScopeProof>
            + drizzle_core::MarkerAggValidFor<Q::Grouped, AggProof>,
    {
        InsertBuilder::from_sql(self.sql.append(query.into_select_query().into_select_sql()))
    }

    /// Appends any SQL as the row source for the chosen columns.
    ///
    /// Only the column list is checked (it must include every required
    /// column). The SQL itself is not checked.
    pub fn select_raw<Q, RequiredProof>(
        self,
        query: Q,
    ) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        Table: InsertSelectTable,
        Targets: IncludesRequired<Table::RequiredColumns, RequiredProof>,
        Q: ToSQL<'a, MySQLValue<'a>>,
    {
        InsertBuilder::from_sql(self.sql.append(query.into_sql()))
    }
}
