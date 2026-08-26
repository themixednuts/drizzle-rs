//! MySQL SQL fragments used by the typed query builder.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
use crate::{traits::MySQLTable, values::MySQLValue};
use drizzle_core::{
    ColumnRef, SQL, SQLChunk, SQLIndex, SQLIndexInfo, SQLTableInfo, ToSQL, Token, helpers,
    traits::SQLModel,
};

pub use drizzle_core::Join;
pub(crate) use helpers::{
    delete, from, group_by_expr, having, limit, order_by, select, select_distinct, set, update,
    r#where,
};

fn join_internal<'a, Table>(
    table: Table,
    join: Join,
    condition: impl ToSQL<'a, MySQLValue<'a>>,
) -> SQL<'a, MySQLValue<'a>>
where
    Table: MySQLTable<'a>,
{
    join.into_sql()
        .append(table.into_sql())
        .push(Token::ON)
        .append(condition.into_sql())
}

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

drizzle_core::impl_join_arg_trait!(
    table_trait: MySQLTable<'a>,
    table_info_trait: SQLTableInfo,
    condition_trait: JoinCondition<'a>,
    value_type: MySQLValue<'a>,
);

mod index_hint_private {
    pub trait Sealed {}
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

impl index_hint_private::Sealed for UseIndex {}
impl index_hint_private::Sealed for ForceIndex {}
impl index_hint_private::Sealed for IgnoreIndex {}

#[doc(hidden)]
pub trait IndexHintKind: index_hint_private::Sealed {
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

/// A table source carrying one typed MySQL index hint.
///
/// Values of this type are created through [`MySQLIndexHintExt`].
#[derive(Debug, Clone, Copy)]
pub struct IndexHintedTable<Table, Index, Kind> {
    table: Table,
    index: Index,
    kind: core::marker::PhantomData<Kind>,
}

impl<Table, Index, Kind> IndexHintedTable<Table, Index, Kind> {
    fn into_sql<'a>(self) -> SQL<'a, MySQLValue<'a>>
    where
        Table: MySQLTable<'a>,
        Index: SQLIndex<'a, crate::common::MySQLSchemaType, MySQLValue<'a>, Table = Table>,
        Kind: IndexHintKind,
    {
        self.table
            .into_sql()
            .append(SQL::raw(Kind::SQL))
            .append(SQL::ident(SQLIndexInfo::name(&self.index)).parens())
    }
}

/// Adds a typed MySQL index hint to a joined table source.
///
/// Use the matching methods on a select builder for its base table. The
/// index's generated metadata must name the same table, so hints cannot be
/// accidentally applied across tables.
pub trait MySQLIndexHintExt: Sized {
    fn use_index<Index>(self, index: Index) -> IndexHintedTable<Self, Index, UseIndex>
    where
        Index: for<'a> SQLIndex<'a, crate::common::MySQLSchemaType, MySQLValue<'a>, Table = Self>,
    {
        IndexHintedTable {
            table: self,
            index,
            kind: core::marker::PhantomData,
        }
    }

    fn force_index<Index>(self, index: Index) -> IndexHintedTable<Self, Index, ForceIndex>
    where
        Index: for<'a> SQLIndex<'a, crate::common::MySQLSchemaType, MySQLValue<'a>, Table = Self>,
    {
        IndexHintedTable {
            table: self,
            index,
            kind: core::marker::PhantomData,
        }
    }

    fn ignore_index<Index>(self, index: Index) -> IndexHintedTable<Self, Index, IgnoreIndex>
    where
        Index: for<'a> SQLIndex<'a, crate::common::MySQLSchemaType, MySQLValue<'a>, Table = Self>,
    {
        IndexHintedTable {
            table: self,
            index,
            kind: core::marker::PhantomData,
        }
    }
}

impl<Table> MySQLIndexHintExt for Table where Table: for<'a> MySQLTable<'a> {}

pub(crate) fn index_hint<'a, Table, Index, Kind>(index: &Index) -> SQL<'a, MySQLValue<'a>>
where
    Table: MySQLTable<'a>,
    Index: SQLIndex<'a, crate::common::MySQLSchemaType, MySQLValue<'a>, Table = Table>,
    Kind: IndexHintKind,
{
    SQL::raw(Kind::SQL).append(SQL::ident(SQLIndexInfo::name(index)).parens())
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

impl<'a, Joined, Index, Kind, From> JoinArg<'a, From> for IndexHintedTable<Joined, Index, Kind>
where
    Joined: MySQLTable<'a> + drizzle_core::Joinable<From> + Default,
    From: SQLTableInfo + Default,
    Index: SQLIndex<'a, crate::common::MySQLSchemaType, MySQLValue<'a>, Table = Joined>,
    Kind: IndexHintKind,
{
    type JoinedTable = Joined;

    fn into_join_sql(self, join: Join) -> SQL<'a, MySQLValue<'a>> {
        join.into_sql()
            .append(self.into_sql())
            .push(Token::ON)
            .append(auto_join_condition::<Joined, From>())
    }
}

impl<'a, Joined, Index, Kind, Condition, From> JoinArg<'a, From>
    for (IndexHintedTable<Joined, Index, Kind>, Condition)
where
    Joined: MySQLTable<'a>,
    Index: SQLIndex<'a, crate::common::MySQLSchemaType, MySQLValue<'a>, Table = Joined>,
    Kind: IndexHintKind,
    Condition: JoinCondition<'a>,
{
    type JoinedTable = Joined;

    fn into_join_sql(self, join: Join) -> SQL<'a, MySQLValue<'a>> {
        let (table, condition) = self;
        join.into_sql()
            .append(table.into_sql())
            .push(Token::ON)
            .append(condition.into_sql())
    }
}

fn columns_info_to_sql<'a>(columns: &[ColumnRef]) -> SQL<'a, MySQLValue<'a>> {
    let mut sql = SQL::with_capacity_chunks(columns.len().saturating_mul(2));
    for (index, column) in columns.iter().enumerate() {
        if index > 0 {
            sql.push_mut(Token::COMMA);
        }
        sql.append_mut(SQL::ident(column.name));
    }
    sql
}

pub(crate) fn insert<'a, Table>(table: &Table) -> SQL<'a, MySQLValue<'a>>
where
    Table: MySQLTable<'a>,
{
    helpers::insert::<Table, crate::common::MySQLSchemaType, MySQLValue<'a>>(table)
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

    columns_info_to_sql(columns.as_ref())
        .parens()
        .push(Token::VALUES)
        .append(row_sql)
}

pub(crate) fn standalone_offset<'a, P>(offset: P) -> SQL<'a, MySQLValue<'a>>
where
    P: drizzle_core::PaginationArg<'a, MySQLValue<'a>>,
{
    SQL::from(Token::LIMIT)
        .append(SQL::raw("18446744073709551615"))
        .append(helpers::offset(offset))
}

fn unqualified_columns<'a>(mut columns: SQL<'a, MySQLValue<'a>>) -> SQL<'a, MySQLValue<'a>> {
    for chunk in &mut columns.chunks {
        if let SQLChunk::Column(column) = chunk {
            *chunk = SQLChunk::ident_static(column.name);
        }
    }
    columns
}

/// A typed MySQL ordering expression that preserves its operand until the
/// builder knows whether table qualification is legal.
#[derive(Debug, Clone, Copy)]
pub struct OrderExpr<T> {
    value: T,
    direction: drizzle_core::OrderBy,
}

impl<'a, T> ToSQL<'a, MySQLValue<'a>> for OrderExpr<T>
where
    T: ToSQL<'a, MySQLValue<'a>>,
{
    fn to_sql(&self) -> SQL<'a, MySQLValue<'a>> {
        self.value.to_sql().append(&self.direction)
    }
}

/// Creates an ascending MySQL ORDER BY expression.
pub const fn asc<T>(value: T) -> OrderExpr<T> {
    OrderExpr {
        value,
        direction: drizzle_core::OrderBy::Asc,
    }
}

/// Creates a descending MySQL ORDER BY expression.
pub const fn desc<T>(value: T) -> OrderExpr<T> {
    OrderExpr {
        value,
        direction: drizzle_core::OrderBy::Desc,
    }
}

/// A compound-query output alias usable in the global ORDER BY clause.
#[derive(Debug, Clone, Copy)]
pub struct OutputAlias(&'static str);

impl<'a> ToSQL<'a, MySQLValue<'a>> for OutputAlias {
    fn to_sql(&self) -> SQL<'a, MySQLValue<'a>> {
        SQL::ident(self.0)
    }
}

/// Names an aliased SELECT output for compound-query ordering.
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

    impl<'a, Cols, Scope, Item, Table, Proof> ProjectionAllows<'a, Item, Table, Proof>
        for drizzle_core::Scoped<drizzle_core::SelectCols<Cols>, Scope>
    where
        Cols: drizzle_core::row::SelectedExpressionList,
        <Cols as drizzle_core::row::SelectedExpressionList>::Expressions:
            drizzle_core::row::ScopeContains<Item, Proof>,
    {
    }

    impl<'a, Scope, Item, Table> ProjectionAllows<'a, Item, Table, ()>
        for drizzle_core::Scoped<drizzle_core::SelectStar, Scope>
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

pub(crate) fn set_order_by<'a, Projection, Table, Proof>(
    order: impl SetOrderBy<'a, Projection, Table, Proof>,
) -> SQL<'a, MySQLValue<'a>> {
    SQL::from_iter([Token::ORDER, Token::BY]).append(order.into_set_order_sql())
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
