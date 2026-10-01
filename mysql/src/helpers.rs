//! `MySQL` helpers used with the query builder: ORDER BY terms
//! ([`asc`], [`desc`], [`output_alias`]) and index hints for joined tables
//! ([`MySQLIndexHintExt`]).
//!
//! Examples use the `drizzle` crate and are not compiled here; see the
//! [`builder`](crate::builder) module.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
use crate::{traits::MySQLTable, values::MySQLValue};
use drizzle_core::{
    SQL, SQLChunk, SQLIndex, SQLIndexInfo, SQLTableInfo, ToSQL, Token, helpers, traits::SQLModel,
};

pub use drizzle_core::Join;
pub(crate) use helpers::{
    delete, from, group_by_expr, having, limit, order_by, select, select_distinct, set, update,
    r#where,
};

/// A typed boolean expression accepted after a MySQL JOIN ... ON clause.
#[doc(hidden)]
pub trait JoinCondition<'a>:
    join_condition_private::Sealed<'a> + ToSQL<'a, MySQLValue<'a>>
{
}

mod join_condition_private {
    pub trait Sealed<'a> {}

    impl<'a, T> Sealed<'a> for T
    where
        T: drizzle_core::expr::Expr<'a, crate::values::MySQLValue<'a>>,
        T::SQLType: drizzle_core::types::BooleanLike,
    {
    }
}

impl<'a, T> JoinCondition<'a> for T
where
    T: drizzle_core::expr::Expr<'a, MySQLValue<'a>>,
    T::SQLType: drizzle_core::types::BooleanLike,
{
}

/// A table-like source accepted by an explicit JOIN tuple.
#[doc(hidden)]
pub trait JoinSource<'a>: join_source_private::Sealed {
    type JoinedTable;

    fn into_join_source_sql(self) -> SQL<'a, MySQLValue<'a>>;
}

mod join_source_private {
    pub trait Sealed {}
}

impl<'a, Table> join_source_private::Sealed for Table where Table: MySQLTable<'a> {}

impl<'a, Name, Projection, Query> join_source_private::Sealed
    for drizzle_core::Derived<'a, MySQLValue<'a>, Name, Projection, Query>
where
    Name: drizzle_core::Tag,
    Projection: drizzle_core::DerivedProjection<Name>,
    Query: ToSQL<'a, MySQLValue<'a>>,
{
}

impl<'a, Table> JoinSource<'a> for Table
where
    Table: MySQLTable<'a>,
{
    type JoinedTable = Table;

    fn into_join_source_sql(self) -> SQL<'a, MySQLValue<'a>> {
        self.into_sql()
    }
}

impl<'a, Name, Projection, Query> JoinSource<'a>
    for drizzle_core::Derived<'a, MySQLValue<'a>, Name, Projection, Query>
where
    Name: drizzle_core::Tag,
    Projection: drizzle_core::DerivedProjection<Name>,
    Query: ToSQL<'a, MySQLValue<'a>>,
{
    type JoinedTable = Self;

    fn into_join_source_sql(self) -> SQL<'a, MySQLValue<'a>> {
        self.into_sql()
    }
}

/// A source or legacy tuple accepted by [`crate::builder::SelectBuilder::cross_join`].
///
/// A bare source renders `CROSS JOIN`. The legacy `(source, predicate)`
/// form renders the equivalent portable `INNER JOIN ... ON ...`, because
/// PostgreSQL does not allow an `ON` clause after `CROSS JOIN`.
#[doc(hidden)]
pub trait CrossJoinArg<'a, FromTable>: cross_join_arg_private::Sealed {
    type JoinedTable;
    /// Sources read by the legacy `ON` predicate (see [`drizzle_core::scope`]).
    type OnSources;

    fn into_cross_join_sql(self) -> SQL<'a, MySQLValue<'a>>;
}

mod cross_join_arg_private {
    pub trait Sealed {}

    impl<'a, Source> Sealed for Source where Source: super::JoinSource<'a> {}

    impl<'a, Source, Condition> Sealed for (Source, Condition)
    where
        Source: super::JoinSource<'a>,
        Condition: super::JoinCondition<'a>,
    {
    }
}

impl<'a, Source, FromTable> CrossJoinArg<'a, FromTable> for Source
where
    Source: JoinSource<'a>,
{
    type JoinedTable = Source::JoinedTable;
    type OnSources = ();

    fn into_cross_join_sql(self) -> SQL<'a, MySQLValue<'a>> {
        Join::new()
            .cross()
            .into_sql()
            .append(self.into_join_source_sql())
    }
}

impl<'a, Source, Condition, FromTable> CrossJoinArg<'a, FromTable> for (Source, Condition)
where
    Source: JoinSource<'a>,
    Condition: JoinCondition<'a> + drizzle_core::expr::ExprSources,
{
    type JoinedTable = Source::JoinedTable;
    type OnSources = Condition::Sources;

    fn into_cross_join_sql(self) -> SQL<'a, MySQLValue<'a>> {
        let (source, condition) = self;
        Join::new()
            .inner()
            .into_sql()
            .append(source.into_join_source_sql())
            .push(Token::ON)
            .append(condition.into_sql())
    }
}

drizzle_core::impl_join_arg_trait!(
    table_trait: MySQLTable<'a>,
    table_info_trait: SQLTableInfo,
    condition_trait: JoinCondition<'a>,
    join_source_trait: JoinSource<'a>,
    value_type: MySQLValue<'a>,
);

mod index_hint_private {
    pub trait Kind {}
    pub trait List<'a, Table> {}
}

/// Marker for a MySQL `USE INDEX` hint.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct UseIndex;

/// Marker for a MySQL `FORCE INDEX` hint.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ForceIndex;

/// Marker for a MySQL `IGNORE INDEX` hint.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct IgnoreIndex;

impl index_hint_private::Kind for UseIndex {}
impl index_hint_private::Kind for ForceIndex {}
impl index_hint_private::Kind for IgnoreIndex {}

#[doc(hidden)]
pub trait IndexHintKind: index_hint_private::Kind {
    const SQL: &'static str;
}

impl IndexHintKind for UseIndex {
    const SQL: &'static str = "USE INDEX ";
}

impl IndexHintKind for ForceIndex {
    const SQL: &'static str = "FORCE INDEX ";
}

impl IndexHintKind for IgnoreIndex {
    const SQL: &'static str = "IGNORE INDEX ";
}

/// One or more generated indexes belonging to the same MySQL table.
#[doc(hidden)]
pub trait IndexHintList<'a, Table>: index_hint_private::List<'a, Table> {
    fn names(&self) -> SQL<'a, MySQLValue<'a>>;
}

impl<'a, Table, Index> index_hint_private::List<'a, Table> for Index where
    Index: SQLIndex<'a, crate::common::MySQLSchemaType, MySQLValue<'a>, Table = Table>
{
}

impl<'a, Table, Index> IndexHintList<'a, Table> for Index
where
    Index: SQLIndex<'a, crate::common::MySQLSchemaType, MySQLValue<'a>, Table = Table>,
{
    fn names(&self) -> SQL<'a, MySQLValue<'a>> {
        SQL::ident(SQLIndexInfo::name(self))
    }
}

macro_rules! index_hint_tuple {
    ($($index:ident: $field:tt),+) => {
        impl<'a, Table, $($index),+> index_hint_private::List<'a, Table> for ($($index,)+)
        where
            $($index: SQLIndex<'a, crate::common::MySQLSchemaType, MySQLValue<'a>, Table = Table>,)+
        {
        }

        impl<'a, Table, $($index),+> IndexHintList<'a, Table> for ($($index,)+)
        where
            $($index: SQLIndex<'a, crate::common::MySQLSchemaType, MySQLValue<'a>, Table = Table>,)+
        {
            fn names(&self) -> SQL<'a, MySQLValue<'a>> {
                SQL::join(
                    [$(SQL::ident(SQLIndexInfo::name(&self.$field)),)+],
                    Token::COMMA,
                )
            }
        }
    };
}

index_hint_tuple!(A: 0, B: 1);
index_hint_tuple!(A: 0, B: 1, C: 2);
index_hint_tuple!(A: 0, B: 1, C: 2, D: 3);
index_hint_tuple!(A: 0, B: 1, C: 2, D: 3, E: 4);
index_hint_tuple!(A: 0, B: 1, C: 2, D: 3, E: 4, F: 5);
index_hint_tuple!(A: 0, B: 1, C: 2, D: 3, E: 4, F: 5, G: 6);
index_hint_tuple!(A: 0, B: 1, C: 2, D: 3, E: 4, F: 5, G: 6, H: 7);

/// A table with one index hint, ready to be joined.
///
/// Created by the methods of [`MySQLIndexHintExt`].
#[derive(Debug, Clone, Copy)]
pub struct IndexHintedTable<Table, Indexes, Kind> {
    table: Table,
    indexes: Indexes,
    kind: core::marker::PhantomData<Kind>,
}

impl<Table, Indexes, Kind> join_source_private::Sealed for IndexHintedTable<Table, Indexes, Kind> {}

impl<Table, Indexes, Kind> IndexHintedTable<Table, Indexes, Kind> {
    fn into_sql<'a>(self) -> SQL<'a, MySQLValue<'a>>
    where
        Table: MySQLTable<'a>,
        Indexes: IndexHintList<'a, Table>,
        Kind: IndexHintKind,
    {
        self.table
            .into_sql()
            .append(SQL::raw(Kind::SQL))
            .append(self.indexes.names().parens())
    }
}

impl<'a, Table, Indexes, Kind> JoinSource<'a> for IndexHintedTable<Table, Indexes, Kind>
where
    Table: MySQLTable<'a>,
    Indexes: IndexHintList<'a, Table>,
    Kind: IndexHintKind,
{
    type JoinedTable = Table;

    fn into_join_source_sql(self) -> SQL<'a, MySQLValue<'a>> {
        self.into_sql()
    }
}

/// Adds an index hint to a table you are about to join.
///
/// Implemented for every `MySQL` table. For the FROM table, use the
/// `use_index` / `force_index` / `ignore_index` methods on the select
/// builder instead. Each index must belong to the hinted table; an index of
/// another table does not compile. Pass one index or a tuple of up to eight.
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
///     .select((users.id, posts.id))
///     .from(users)
///     .inner_join((posts.use_index(PostsUserIdIdx::new()), eq(posts.user_id, users.id)));
/// assert_eq!(
///     query.to_sql().sql(),
///     "SELECT `users`.`id`, `posts`.`id` FROM `users` INNER JOIN `posts` USE INDEX (`posts_user_id_idx`) ON `posts`.`user_id` = `users`.`id`"
/// );
/// # "####;
/// ```
pub trait MySQLIndexHintExt: Sized {
    /// Adds `USE INDEX (..)`: `MySQL` only considers these indexes.
    fn use_index<Indexes>(self, indexes: Indexes) -> IndexHintedTable<Self, Indexes, UseIndex>
    where
        Indexes: for<'a> IndexHintList<'a, Self>,
    {
        IndexHintedTable {
            table: self,
            indexes,
            kind: core::marker::PhantomData,
        }
    }

    /// Adds `FORCE INDEX (..)`: `MySQL` uses a table scan only if none of
    /// these indexes can be used.
    fn force_index<Indexes>(self, indexes: Indexes) -> IndexHintedTable<Self, Indexes, ForceIndex>
    where
        Indexes: for<'a> IndexHintList<'a, Self>,
    {
        IndexHintedTable {
            table: self,
            indexes,
            kind: core::marker::PhantomData,
        }
    }

    /// Adds `IGNORE INDEX (..)`: `MySQL` does not use these indexes.
    fn ignore_index<Indexes>(self, indexes: Indexes) -> IndexHintedTable<Self, Indexes, IgnoreIndex>
    where
        Indexes: for<'a> IndexHintList<'a, Self>,
    {
        IndexHintedTable {
            table: self,
            indexes,
            kind: core::marker::PhantomData,
        }
    }
}

impl<Table> MySQLIndexHintExt for Table where Table: for<'a> MySQLTable<'a> {}

pub(crate) fn index_hint<'a, Table, Indexes, Kind>(indexes: &Indexes) -> SQL<'a, MySQLValue<'a>>
where
    Table: MySQLTable<'a>,
    Indexes: IndexHintList<'a, Table>,
    Kind: IndexHintKind,
{
    SQL::raw(Kind::SQL).append(indexes.names().parens())
}

fn auto_join_condition<'a, Joined, From>() -> SQL<'a, MySQLValue<'a>>
where
    Joined: MySQLTable<'a> + drizzle_core::Joinable<From> + Default,
    From: SQLTableInfo + Default,
{
    let joined = Joined::default();
    let from = From::default();
    let columns = <Joined as drizzle_core::Joinable<From>>::fk_columns();
    let mut condition = SQL::with_capacity_chunks(columns.len().saturating_mul(7));
    for (index, (joined_column, from_column)) in columns.iter().enumerate() {
        if index > 0 {
            condition.push_mut(Token::AND);
        }
        condition.append_mut(
            SQL::ident(joined.name())
                .push(Token::DOT)
                .append(SQL::ident(*joined_column)),
        );
        condition.push_mut(Token::EQ);
        condition.append_mut(
            SQL::ident(from.name())
                .push(Token::DOT)
                .append(SQL::ident(*from_column)),
        );
    }
    condition
}

impl<'a, Joined, Indexes, Kind, From> JoinArg<'a, From> for IndexHintedTable<Joined, Indexes, Kind>
where
    Joined: MySQLTable<'a> + drizzle_core::Joinable<From> + Default,
    From: SQLTableInfo + Default,
    Indexes: IndexHintList<'a, Joined>,
    Kind: IndexHintKind,
{
    type JoinedTable = Joined;
    type OnSources = ();

    fn into_join_sql(self, join: Join) -> SQL<'a, MySQLValue<'a>> {
        join.into_sql()
            .append(self.into_sql())
            .push(Token::ON)
            .append(auto_join_condition::<Joined, From>())
    }
}

pub(crate) fn insert_ignore<'a>(mut insert: SQL<'a, MySQLValue<'a>>) -> SQL<'a, MySQLValue<'a>> {
    debug_assert!(matches!(
        insert.chunks.first(),
        Some(SQLChunk::Token(Token::INSERT))
    ));
    insert.chunks.insert(1, SQLChunk::Token(Token::IGNORE));
    insert
}

pub(crate) fn on_duplicate_key_update<'a, Table>(
    assignments: &Table::Update,
) -> SQL<'a, MySQLValue<'a>>
where
    Table: MySQLTable<'a>,
{
    let assignment_sql = assignments.to_sql();
    assert!(
        !assignment_sql.chunks.is_empty(),
        "on_duplicate_key_update requires at least one assignment"
    );
    SQL::from_iter([Token::ON, Token::DUPLICATE, Token::KEY, Token::UPDATE]).append(assignment_sql)
}

#[track_caller]
pub(crate) fn values<'a, Table, T>(
    rows: impl IntoIterator<Item = Table::Insert<T>>,
) -> SQL<'a, MySQLValue<'a>>
where
    Table: MySQLTable<'a>,
{
    let rows: Vec<_> = rows.into_iter().collect();
    assert!(!rows.is_empty(), "insert values requires at least one row");

    let columns = rows[0].columns();
    // A `None` passed to a `with_*` setter leaves that column to its default
    // without changing the row's type, so rows can set different columns.
    // Then every row lists the union of the columns, with `DEFAULT` where it
    // sets none.
    if rows[1..]
        .iter()
        .any(|row| row.columns().as_ref() != columns.as_ref())
        && let Some(rows_sql) = drizzle_core::helpers::insert_values_with_defaults(
            rows.iter()
                .map(|row| (row.columns(), row.values()))
                .collect(),
        )
    {
        return rows_sql;
    }

    if columns.is_empty() {
        let mut row_sql = SQL::with_capacity_chunks(rows.len().saturating_mul(3));
        for index in 0..rows.len() {
            if index > 0 {
                row_sql.push_mut(Token::COMMA);
            }
            row_sql.append_mut(SQL::empty().parens());
        }
        return SQL::empty().parens().push(Token::VALUES).append(row_sql);
    }

    let mut row_sql = SQL::with_capacity_chunks(rows.len().saturating_mul(4));
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            row_sql.push_mut(Token::COMMA);
        }
        row_sql.push_mut(Token::LPAREN);
        row_sql.append_mut(row.values());
        row_sql.push_mut(Token::RPAREN);
    }

    SQL::columns(columns.as_ref())
        .parens()
        .push(Token::VALUES)
        .append(row_sql)
}

pub(crate) fn standalone_offset<'a, P>(offset: P) -> SQL<'a, MySQLValue<'a>>
where
    P: drizzle_core::PaginationArg<'a, MySQLValue<'a>>,
{
    SQL::from(Token::LIMIT)
        .append(SQL::raw(drizzle_core::helpers::MYSQL_UNBOUNDED_LIMIT))
        .append(helpers::offset(offset))
}

pub(crate) fn unqualified_columns<'a>(
    mut columns: SQL<'a, MySQLValue<'a>>,
) -> SQL<'a, MySQLValue<'a>> {
    for chunk in &mut columns.chunks {
        if let SQLChunk::Column(column) = chunk {
            *chunk = SQLChunk::ident_static(column.name);
        }
    }
    columns
}

/// An ORDER BY term with a direction, made by [`asc`] or [`desc`].
///
/// It keeps its operand unrendered so that, after a set operation, the
/// builder can write a column without its table name.
#[derive(Debug, Clone, Copy)]
pub struct OrderExpr<T> {
    value: T,
    direction: drizzle_core::OrderBy,
}

impl<T: drizzle_core::expr::ExprSources> drizzle_core::expr::ExprSources for OrderExpr<T> {
    type Sources = T::Sources;
}

impl<T: drizzle_core::expr::ExprSources> drizzle_core::OrderTerm for OrderExpr<T> {}

/// An output alias names a SELECT output, not a FROM source.
impl drizzle_core::expr::ExprSources for OutputAlias {
    type Sources = ();
}

impl<'a, T> ToSQL<'a, MySQLValue<'a>> for OrderExpr<T>
where
    T: ToSQL<'a, MySQLValue<'a>>,
{
    fn to_sql(&self) -> SQL<'a, MySQLValue<'a>> {
        self.value.to_sql().append(&self.direction)
    }
}

/// Sorts by `value` in ascending order (`value ASC`).
pub const fn asc<T>(value: T) -> OrderExpr<T> {
    OrderExpr {
        value,
        direction: drizzle_core::OrderBy::Asc,
    }
}

/// Sorts by `value` in descending order (`value DESC`).
pub const fn desc<T>(value: T) -> OrderExpr<T> {
    OrderExpr {
        value,
        direction: drizzle_core::OrderBy::Desc,
    }
}

/// The name of a SELECT output, for the ORDER BY of a set operation.
///
/// Made by [`output_alias`].
#[derive(Debug, Clone, Copy)]
pub struct OutputAlias(&'static str);

impl<'a> ToSQL<'a, MySQLValue<'a>> for OutputAlias {
    fn to_sql(&self) -> SQL<'a, MySQLValue<'a>> {
        SQL::ident(self.0)
    }
}

/// Refers to a named SELECT output in the ORDER BY of a set operation.
///
/// Give the output its name with `alias(expr, "name")` in the first
/// query. The name is not checked against the query, so a typo is only
/// caught by `MySQL`. See the set-operation `order_by` on
/// [`SelectBuilder`](crate::builder::SelectBuilder) for an example.
#[must_use]
pub const fn output_alias(name: &'static str) -> OutputAlias {
    OutputAlias(name)
}

#[doc(hidden)]
pub trait SetOrderBy<'a, Projection, Table, Proof>:
    set_order_private::Sealed<'a, Projection, Table, Proof>
{
    fn into_set_order_sql(self) -> SQL<'a, MySQLValue<'a>>;
}

mod set_order_private {
    use super::{MySQLValue, OrderExpr, OutputAlias, ToSQL};

    pub trait ProjectionAllows<'a, Item, Table, Proof> {}

    impl<'a, Cols, Scope, Used, Item, Table, Proof> ProjectionAllows<'a, Item, Table, Proof>
        for drizzle_core::Scoped<drizzle_core::SelectCols<Cols>, Scope, Used>
    where
        Cols: drizzle_core::row::SelectedExpressionList,
        <Cols as drizzle_core::row::SelectedExpressionList>::Expressions:
            drizzle_core::scope::ListContains<Item, Proof>,
    {
    }

    impl<'a, Scope, Used, Item, Table> ProjectionAllows<'a, Item, Table, ()>
        for drizzle_core::Scoped<drizzle_core::SelectStar, Scope, Used>
    where
        Item: drizzle_core::traits::SQLColumn<'a, MySQLValue<'a>>
            + drizzle_core::traits::ColumnOf<Table>,
    {
    }

    pub trait ProjectionListAllowed<'a, Projection, Table, Proof> {}

    impl<'a, Projection, Table> ProjectionListAllowed<'a, Projection, Table, ()> for drizzle_core::Nil {}

    impl<'a, Projection, Table, Head, Tail, HeadProof, TailProof>
        ProjectionListAllowed<'a, Projection, Table, (HeadProof, TailProof)>
        for drizzle_core::Cons<Head, Tail>
    where
        Head: drizzle_core::traits::SQLColumn<'a, MySQLValue<'a>>,
        Projection: ProjectionAllows<'a, Head, Table, HeadProof>,
        Tail: ProjectionListAllowed<'a, Projection, Table, TailProof>,
    {
    }

    pub trait Sealed<'a, Projection, Table, Proof> {}

    impl<'a, Projection, Table, Columns, Cols, Proof> Sealed<'a, Projection, Table, Proof> for Columns
    where
        Columns: ToSQL<'a, MySQLValue<'a>>
            + drizzle_core::IntoSelectTarget<Marker = drizzle_core::SelectCols<Cols>>,
        Cols: drizzle_core::row::SelectedExpressionList,
        <Cols as drizzle_core::row::SelectedExpressionList>::Expressions:
            ProjectionListAllowed<'a, Projection, Table, Proof>,
    {
    }

    impl<'a, Projection, Table, Column, Proof> Sealed<'a, Projection, Table, Proof>
        for OrderExpr<Column>
    where
        Column: drizzle_core::traits::SQLColumn<'a, MySQLValue<'a>>,
        Projection: ProjectionAllows<'a, Column, Table, Proof>,
    {
    }

    impl<'a, Projection, Table> Sealed<'a, Projection, Table, ()> for OutputAlias {}
    impl<'a, Projection, Table> Sealed<'a, Projection, Table, ()> for OrderExpr<OutputAlias> {}
}

impl<'a, Projection, Table, Columns, Cols, Proof> SetOrderBy<'a, Projection, Table, Proof>
    for Columns
where
    Columns: ToSQL<'a, MySQLValue<'a>>
        + drizzle_core::IntoSelectTarget<Marker = drizzle_core::SelectCols<Cols>>,
    Cols: drizzle_core::row::SelectedExpressionList,
    <Cols as drizzle_core::row::SelectedExpressionList>::Expressions:
        set_order_private::ProjectionListAllowed<'a, Projection, Table, Proof>,
{
    fn into_set_order_sql(self) -> SQL<'a, MySQLValue<'a>> {
        unqualified_columns(self.into_sql())
    }
}

impl<'a, Projection, Table, Column, Proof> SetOrderBy<'a, Projection, Table, Proof>
    for OrderExpr<Column>
where
    Column: drizzle_core::traits::SQLColumn<'a, MySQLValue<'a>>,
    Projection: set_order_private::ProjectionAllows<'a, Column, Table, Proof>,
{
    fn into_set_order_sql(self) -> SQL<'a, MySQLValue<'a>> {
        unqualified_columns(self.value.into_sql()).append(&self.direction)
    }
}

impl<'a, Projection, Table> SetOrderBy<'a, Projection, Table, ()> for OutputAlias {
    fn into_set_order_sql(self) -> SQL<'a, MySQLValue<'a>> {
        self.to_sql()
    }
}

impl<'a, Projection, Table> SetOrderBy<'a, Projection, Table, ()> for OrderExpr<OutputAlias> {
    fn into_set_order_sql(self) -> SQL<'a, MySQLValue<'a>> {
        self.value.to_sql().append(&self.direction)
    }
}

pub(crate) fn set_op<'a>(
    left: SQL<'a, MySQLValue<'a>>,
    operator: Token,
    all: bool,
    right: SQL<'a, MySQLValue<'a>>,
) -> SQL<'a, MySQLValue<'a>> {
    let sql = left.parens().push(operator);
    let sql = if all { sql.push(Token::ALL) } else { sql };
    sql.append(right.parens())
}
