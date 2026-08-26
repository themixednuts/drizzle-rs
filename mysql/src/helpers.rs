//! MySQL SQL fragments used by the typed query builder.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
use crate::{traits::MySQLTable, values::MySQLValue};
use drizzle_core::{
    ColumnRef, SQL, SQLChunk, SQLTableInfo, ToSQL, Token, helpers, traits::SQLModel,
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
