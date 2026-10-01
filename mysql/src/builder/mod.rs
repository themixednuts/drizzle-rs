//! Typed `MySQL` query builder.
//!
//! Start with [`QueryBuilder::new`]. Each statement has its own builder
//! alias with its clause order: [`SelectBuilder`], [`InsertBuilder`],
//! [`UpdateBuilder`] and [`DeleteBuilder`].
//!
//! Examples in this module use the `drizzle` crate and its macros, which
//! this crate cannot depend on, so they are shown but not compiled here.
//! The same queries are compiled and checked in the workspace's
//! `tests/mysql/builder.rs`.

use crate::{common::MySQLSchemaType, traits::MySQLTable, values::MySQLValue};
use core::{fmt::Debug, marker::PhantomData};
use drizzle_core::{SQL, ToSQL, Token};

pub use drizzle_core::{BuilderInit, ExecutableState};

macro_rules! mutation_builder_methods {
    (
        $builder:ident,
        prepare: [$($prepare:ty),+ $(,)?],
        order_by: [$($order:ty),+ $(,)?] => $ordered:ty,
        limit: [$($limit:ty),+ $(,)?] => $limited:ty $(,)?
    ) => {
        $(
            impl<'a, S, T, M, R> $builder<'a, S, $prepare, T, M, R> {
                /// Renders this statement as a [`PreparedStatement`](drizzle_core::prepared::PreparedStatement)
                /// whose placeholders are bound in `MySQL`'s positional order.
                #[must_use]
                pub fn prepare(
                    &self,
                ) -> drizzle_core::prepared::PreparedStatement<'a, crate::values::MySQLValue<'a>> {
                    self.prepared_statement()
                }
            }
        )+

        $(
            impl<'a, S, T> $builder<'a, S, $order, T> {
                /// Adds ORDER BY, which sets the order rows are changed in.
                ///
                /// Mostly useful with `limit`. Only columns of the target
                /// table may be used.
                pub fn order_by<O, ScopeProof>(self, order: O) -> $builder<'a, S, $ordered, T>
                where
                    O: drizzle_core::ToSQL<'a, crate::values::MySQLValue<'a>>
                        + drizzle_core::expr::ExprSources,
                    O::Sources: drizzle_core::scope::SourcesIn<
                        drizzle_core::Cons<T, drizzle_core::Nil>,
                        ScopeProof,
                    >,
                {
                    $builder::from_sql(self.sql.append(crate::helpers::order_by(order)))
                }
            }
        )+

        $(
            impl<'a, S, T> $builder<'a, S, $limit, T> {
                /// Adds LIMIT, which caps how many rows are changed.
                ///
                /// # Panics
                ///
                /// Panics when a numeric argument is negative or does not
                /// fit in `usize`.
                #[track_caller]
                pub fn limit<P>(self, limit: P) -> $builder<'a, S, $limited, T>
                where
                    P: drizzle_core::PaginationArg<'a, crate::values::MySQLValue<'a>>,
                {
                    $builder::from_sql(self.sql.append(crate::helpers::limit(limit)))
                }
            }
        )+
    };
}

pub mod cte;
/// Typed MySQL `DELETE` statements.
pub mod delete;
/// Typed MySQL `INSERT` statements.
pub mod insert;
/// Typed MySQL `SELECT` statements.
pub mod select;
/// Typed MySQL `UPDATE` statements.
pub mod update;

pub use cte::{CTEDefinition, CTEView};
pub use delete::{DeleteBuilder, DeleteInitial, DeleteLimitSet, DeleteOrderSet, DeleteWhereSet};
pub use insert::{
    InsertBuilder, InsertColumnsSet, InsertIgnoreSet, InsertInitial, InsertOnDuplicateKeyUpdateSet,
    InsertValuesSet,
};
pub use select::{
    CompletedSelect, ForShare, ForUpdate, IntoSelectQuery, NoWait, SelectBuilder, SelectForSet,
    SelectFromSet, SelectGroupSet, SelectHavingSet, SelectIndexHintSet, SelectInitial,
    SelectJoinSet, SelectLimitSet, SelectOffsetSet, SelectOrderSet, SelectSetOpSet, SelectWhereSet,
    SkipLocked, Wait,
};
pub use update::{
    UpdateBuilder, UpdateInitial, UpdateLimitSet, UpdateOrderSet, UpdateSetClauseSet,
    UpdateWhereSet,
};

/// Builder state after [`QueryBuilder::with`]: the next call starts the
/// statement that uses the common table expressions.
#[derive(Debug, Clone)]
pub struct CTEInit;

/// Type-safe SQL query builder for `MySQL`.
///
/// Start with [`QueryBuilder::new`], then call [`select`](Self::select),
/// [`insert`](Self::insert), [`update`](Self::update),
/// [`delete`](Self::delete) or [`with`](Self::with). Each method returns a
/// builder in a new state that only offers the clauses that may come next,
/// so an out-of-order query does not compile. Call [`ToSQL::to_sql`] for the
/// SQL text and parameters, or `prepare()` for a statement with
/// placeholders. It does not run queries; the `drizzle` crate's `MySQL`
/// drivers do.
///
/// # Type parameters
///
/// - `Schema`: the schema the query runs against.
/// - `State`: which clauses have been added so far.
/// - `Table`: the table the last FROM or JOIN added, or the target table of
///   an INSERT, UPDATE or DELETE.
/// - `Marker`: the selected columns and the tables in scope.
/// - `Row`: the Rust type of one result row.
/// - `Grouped`: the GROUP BY columns.
///
/// # Examples
///
/// ```rust
/// # let _ = r####"
/// use drizzle::core::expr::eq;
/// use drizzle::mysql::{builder::QueryBuilder, prelude::*};
///
/// #[MySQLTable(NAME = "users")]
/// struct Users {
///     #[column(PRIMARY, AUTO_INCREMENT)]
///     id: u64,
///     #[column(VARCHAR(255))]
///     name: String,
///     #[column(DEFAULT = true)]
///     active: bool,
/// }
///
/// #[derive(MySQLSchema)]
/// struct Schema {
///     users: Users,
/// }
///
/// let builder = QueryBuilder::new::<Schema>();
/// let Schema { users } = Schema::new();
/// let query = builder
///     .select(users.name)
///     .from(users)
///     .r#where(eq(users.active, true))
///     .order_by(asc(users.name))
///     .limit(10);
/// assert_eq!(
///     query.to_sql().sql(),
///     "SELECT `users`.`name` FROM `users` WHERE `users`.`active` = ? ORDER BY `users`.`name` ASC LIMIT ?"
/// );
/// # "####;
/// ```
#[derive(Debug, Clone)]
pub struct QueryBuilder<
    'a,
    Schema = (),
    State = (),
    Table = (),
    Marker = (),
    Row = (),
    Grouped = (),
> {
    pub(crate) sql: SQL<'a, MySQLValue<'a>>,
    pub(crate) schema: PhantomData<Schema>,
    pub(crate) state: PhantomData<State>,
    pub(crate) table: PhantomData<Table>,
    pub(crate) marker: PhantomData<Marker>,
    pub(crate) row: PhantomData<Row>,
    pub(crate) grouped: PhantomData<Grouped>,
}

impl<'a, Schema, State, Table, Marker, Row, Grouped> ToSQL<'a, MySQLValue<'a>>
    for QueryBuilder<'a, Schema, State, Table, Marker, Row, Grouped>
where
    State: ExecutableState,
{
    fn to_sql(&self) -> SQL<'a, MySQLValue<'a>> {
        self.sql.clone()
    }

    fn into_sql(self) -> SQL<'a, MySQLValue<'a>> {
        self.sql
    }
}

impl<'a, Schema, State, Table, Marker, Row, Grouped>
    QueryBuilder<'a, Schema, State, Table, Marker, Row, Grouped>
where
    State: ExecutableState,
{
    /// Prepends a [sqlcommenter](https://google.github.io/sqlcommenter/)
    /// comment (`/*...*/`) to the query.
    ///
    /// `/*` and `*/` inside `text` are escaped so the text cannot end the
    /// comment early. An empty `text` leaves the query unchanged.
    #[must_use]
    pub fn comment(mut self, text: impl AsRef<str>) -> Self {
        let fragment = drizzle_core::sql::comment::<MySQLValue<'a>>(text);
        if !fragment.chunks.is_empty() {
            let existing = core::mem::replace(&mut self.sql, fragment);
            self.sql.append_mut(existing);
        }
        self
    }

    /// Prepends a tag-style [sqlcommenter](https://google.github.io/sqlcommenter/)
    /// comment to the query.
    ///
    /// Each `(key, value)` pair is URL-encoded and written as `key='value'`.
    /// Pairs are sorted and joined with `,`. Pairs with an empty value are
    /// skipped; if none remain, the query is unchanged.
    #[must_use]
    pub fn comment_tags<I, K, V>(mut self, pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let fragment = drizzle_core::sql::comment_tags::<MySQLValue<'a>, _, _, _>(pairs);
        if !fragment.chunks.is_empty() {
            let existing = core::mem::replace(&mut self.sql, fragment);
            self.sql.append_mut(existing);
        }
        self
    }
}

impl<'a> QueryBuilder<'a> {
    /// Creates a query builder for the schema `S`.
    #[must_use]
    pub const fn new<S>() -> QueryBuilder<'a, S, BuilderInit> {
        QueryBuilder {
            sql: SQL::empty(),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }
}

impl<'a, Schema> QueryBuilder<'a, Schema, BuilderInit> {
    /// Starts a `SELECT` with the given columns.
    ///
    /// Pass one column or expression, a tuple of them, or `()` to select
    /// every column of the FROM table (and of joined tables). Call
    /// [`from`](select::SelectBuilder::from) next.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # let _ = r####"
    /// # use drizzle::core::expr::eq;
    /// # use drizzle::mysql::{builder::QueryBuilder, prelude::*};
    /// #
    /// # #[MySQLTable(NAME = "users")]
    /// # struct Users {
    /// #     #[column(PRIMARY, AUTO_INCREMENT)]
    /// #     id: u64,
    /// #     #[column(VARCHAR(255))]
    /// #     name: String,
    /// #     #[column(DEFAULT = true)]
    /// #     active: bool,
    /// # }
    /// #
    /// # #[derive(MySQLSchema)]
    /// # struct Schema {
    /// #     users: Users,
    /// # }
    /// #
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { users } = Schema::new();
    /// let query = builder.select(()).from(users);
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     "SELECT `users`.`id`, `users`.`name`, `users`.`active` FROM `users`"
    /// );
    /// # "####;
    /// ```
    pub fn select<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), T::Marker>
    where
        T: ToSQL<'a, MySQLValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        QueryBuilder::from_sql(crate::helpers::select(columns))
    }

    /// Starts a `SELECT DISTINCT`, which drops duplicate rows.
    pub fn select_distinct<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), T::Marker>
    where
        T: ToSQL<'a, MySQLValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        QueryBuilder::from_sql(crate::helpers::select_distinct(columns))
    }

    /// Starts an `INSERT` into `table`.
    ///
    /// Call [`values`](insert::InsertBuilder::values),
    /// [`select`](insert::InsertBuilder::select),
    /// [`columns`](insert::InsertBuilder::columns) or
    /// [`ignore`](insert::InsertBuilder::ignore) next.
    pub fn insert<Table>(
        &self,
        table: Table,
    ) -> insert::InsertBuilder<'a, Schema, insert::InsertInitial, Table>
    where
        Table: MySQLTable<'a>,
    {
        QueryBuilder::from_sql(drizzle_core::helpers::insert::<
            Table,
            MySQLSchemaType,
            MySQLValue<'a>,
        >(&table))
    }

    /// Starts an `UPDATE` of `table`. Call
    /// [`set`](update::UpdateBuilder::set) next.
    pub fn update<Table>(
        &self,
        table: Table,
    ) -> update::UpdateBuilder<'a, Schema, update::UpdateInitial, Table>
    where
        Table: MySQLTable<'a>,
    {
        QueryBuilder::from_sql(crate::helpers::update::<
            Table,
            MySQLSchemaType,
            MySQLValue<'a>,
        >(&table))
    }

    /// Starts a `DELETE` from `table`.
    ///
    /// Without `where`, the statement deletes every row.
    pub fn delete<Table>(
        &self,
        table: Table,
    ) -> delete::DeleteBuilder<'a, Schema, delete::DeleteInitial, Table>
    where
        Table: MySQLTable<'a>,
    {
        QueryBuilder::from_sql(crate::helpers::delete::<
            Table,
            MySQLSchemaType,
            MySQLValue<'a>,
        >(&table))
    }

    /// Starts a WITH clause with one common table expression.
    ///
    /// Build the CTE with [`into_cte`](select::SelectBuilder::into_cte). Add
    /// more with another `.with(..)`, then start a SELECT, UPDATE or DELETE.
    /// For `INSERT ... SELECT`, put the WITH on the inner SELECT instead.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # let _ = r####"
    /// # use drizzle::core::expr::eq;
    /// # use drizzle::mysql::{builder::QueryBuilder, prelude::*};
    /// #
    /// # #[MySQLTable(NAME = "users")]
    /// # struct Users {
    /// #     #[column(PRIMARY, AUTO_INCREMENT)]
    /// #     id: u64,
    /// #     #[column(VARCHAR(255))]
    /// #     name: String,
    /// #     #[column(DEFAULT = true)]
    /// #     active: bool,
    /// # }
    /// #
    /// # #[derive(MySQLSchema)]
    /// # struct Schema {
    /// #     users: Users,
    /// # }
    /// #
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { users } = Schema::new();
    /// struct Active;
    /// impl drizzle::core::Tag for Active {
    ///     const NAME: &'static str = "active";
    /// }
    ///
    /// let active = builder
    ///     .select((users.id, users.name))
    ///     .from(users)
    ///     .r#where(eq(users.active, true))
    ///     .into_cte::<Active>();
    /// let query = builder.with(&active).select(active.name).from(&active);
    /// // WITH `active` AS (SELECT ...) SELECT `active`.`name` FROM `active`
    /// # "####;
    /// ```
    pub fn with<C>(&self, cte: &C) -> QueryBuilder<'a, Schema, CTEInit>
    where
        C: CTEDefinition<'a>,
    {
        QueryBuilder::from_sql(SQL::from(Token::WITH).append(cte.cte_definition()))
    }
}

impl<'a, Schema> QueryBuilder<'a, Schema, CTEInit> {
    /// Starts a `SELECT` after the WITH clause.
    pub fn select<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), T::Marker>
    where
        T: ToSQL<'a, MySQLValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        QueryBuilder::from_sql(self.sql.clone().append(crate::helpers::select(columns)))
    }

    /// Starts a `SELECT DISTINCT` after the WITH clause.
    pub fn select_distinct<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), T::Marker>
    where
        T: ToSQL<'a, MySQLValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        QueryBuilder::from_sql(
            self.sql
                .clone()
                .append(crate::helpers::select_distinct(columns)),
        )
    }

    /// Starts an `UPDATE` after the WITH clause.
    pub fn update<Table>(
        &self,
        table: Table,
    ) -> update::UpdateBuilder<'a, Schema, update::UpdateInitial, Table>
    where
        Table: MySQLTable<'a>,
    {
        QueryBuilder::from_sql(self.sql.clone().append(crate::helpers::update::<
            Table,
            MySQLSchemaType,
            MySQLValue<'a>,
        >(&table)))
    }

    /// Starts a `DELETE` after the WITH clause.
    pub fn delete<Table>(
        &self,
        table: Table,
    ) -> delete::DeleteBuilder<'a, Schema, delete::DeleteInitial, Table>
    where
        Table: MySQLTable<'a>,
    {
        QueryBuilder::from_sql(self.sql.clone().append(crate::helpers::delete::<
            Table,
            MySQLSchemaType,
            MySQLValue<'a>,
        >(&table)))
    }

    #[must_use]
    /// Adds another common table expression to the WITH clause.
    pub fn with<C>(&self, cte: &C) -> Self
    where
        C: CTEDefinition<'a>,
    {
        QueryBuilder::from_sql(
            self.sql
                .clone()
                .push(Token::COMMA)
                .append(cte.cte_definition()),
        )
    }
}

impl<'a, S, State, T, M, R, G> QueryBuilder<'a, S, State, T, M, R, G> {
    pub(crate) fn from_sql(sql: SQL<'a, MySQLValue<'a>>) -> Self {
        Self {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    pub(crate) fn prepared_statement(
        &self,
    ) -> drizzle_core::prepared::PreparedStatement<'a, MySQLValue<'a>> {
        drizzle_core::prepared::prepare_render(&self.sql)
    }
}
