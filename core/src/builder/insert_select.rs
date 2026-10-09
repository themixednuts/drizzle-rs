use core::marker::PhantomData;

use crate::{
    Cons, HasSelectModel, IntoSelectTarget, Nil, SQL, SQLChunk, SQLColumnInfo, SQLParam, Scoped,
    SelectAs, SelectCols, SelectStar, SelectedExpressionList, Token, TypeEq, TypeSet, expr::Expr,
    scope::ListIncludes, types::Assignable,
};

/// INSERT builder state after `.columns(...)`, waiting for its SELECT
/// source. `Columns` is the type-level list of target columns.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertColumnsSet<Columns>(PhantomData<Columns>);

/// A generated table column that may appear in an INSERT target list.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a column of `{Table}`",
    label = "the target columns belong to the table being inserted into"
)]
#[doc(hidden)]
pub trait InsertColumn<Table> {
    /// The column type itself (the same type for a bare column).
    type Column;
}

impl<T, Table> InsertColumn<Table> for &T
where
    T: InsertColumn<Table>,
{
    type Column = T::Column;
}

/// Generated INSERT SELECT metadata for a table.
#[doc(hidden)]
pub trait InsertSelectTable {
    /// Insertable columns, in order.
    type Columns: TypeSet;
    /// Columns an INSERT must set: `NOT NULL` without a database default.
    type RequiredColumns: TypeSet;

    /// Names of the insertable columns, in order.
    const INSERT_COLUMNS: &'static [&'static str];

    /// Renders `("col1", "col2", ...)` from [`Self::INSERT_COLUMNS`].
    fn insert_columns_sql<'a, V: SQLParam>() -> SQL<'a, V> {
        let mut sql = SQL::empty();
        for (index, column) in Self::INSERT_COLUMNS.iter().enumerate() {
            if index > 0 {
                sql.push_mut(Token::COMMA);
            }
            sql.append_mut(SQL::ident(*column));
        }
        sql.parens()
    }
}

impl<T> InsertSelectTable for &T
where
    T: InsertSelectTable,
{
    type Columns = T::Columns;
    type RequiredColumns = T::RequiredColumns;

    const INSERT_COLUMNS: &'static [&'static str] = T::INSERT_COLUMNS;
}

/// A table whose SELECT model has exactly its insertable columns.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "select-all cannot feed this INSERT target",
    label = "select the insertable source columns explicitly"
)]
pub trait InsertSelectAllColumns: InsertSelectTable {}

impl<T> InsertSelectAllColumns for &T where T: InsertSelectAllColumns {}

/// Pairwise compatibility between target columns and SELECT expressions.
#[diagnostic::on_unimplemented(
    message = "the SELECT does not provide one value of a matching type for each INSERT column",
    label = "its columns do not line up with the target columns",
    note = "select one value per target column, in the same order, each assignable to its column"
)]
#[doc(hidden)]
pub trait InsertSelectColumns<'a, V: SQLParam, Source> {}

impl<'a, V: SQLParam> InsertSelectColumns<'a, V, Nil> for Nil {}

impl<'a, V, TargetExpr, TargetTail, SourceExpr, SourceTail>
    InsertSelectColumns<'a, V, Cons<SourceExpr, SourceTail>> for Cons<TargetExpr, TargetTail>
where
    V: SQLParam,
    TargetExpr: Expr<'a, V>,
    SourceExpr: Expr<'a, V>,
    TargetExpr::SQLType: Assignable<SourceExpr::SQLType>,
    TargetExpr::Nullable: crate::expr::AcceptsNullability<SourceExpr::Nullable>,
    TargetTail: InsertSelectColumns<'a, V, SourceTail>,
{
}

/// An explicit column selection that is valid as an INSERT target list.
#[doc(hidden)]
pub trait InsertTargetColumns<'a, V: SQLParam, Table> {
    /// The target columns as a type-level list.
    type Columns: TypeSet;

    /// Renders `("col1", "col2", ...)`.
    ///
    /// # Panics
    ///
    /// Panics if the list is empty or names a column twice.
    fn into_target_columns_sql(self) -> SQL<'a, V>;
}

/// Target columns of an explicit column selection.
#[doc(hidden)]
pub trait InsertTargetMarker<Table> {
    type Columns: TypeSet;
}

impl<Table, Selected> InsertTargetMarker<Table> for SelectCols<Selected>
where
    Selected: SelectedExpressionList,
    Selected::Expressions: InsertTargetColumnList<Table>,
{
    type Columns = <Selected::Expressions as InsertTargetColumnList<Table>>::Columns;
}

/// A type-level list of target columns that all belong to `Table`.
#[doc(hidden)]
pub trait InsertTargetColumnList<Table> {
    /// The list with each element resolved to its column type.
    type Columns: TypeSet;

    /// Appends the quoted column names, comma-separated, to `sql`.
    ///
    /// # Panics
    ///
    /// Panics if a column already appears in `sql`.
    fn append_columns<'a, V: SQLParam>(sql: &mut SQL<'a, V>);
}

impl<Table> InsertTargetColumnList<Table> for Nil {
    type Columns = Nil;

    fn append_columns<'a, V: SQLParam>(_sql: &mut SQL<'a, V>) {}
}

impl<Table, Head, Tail> InsertTargetColumnList<Table> for Cons<Head, Tail>
where
    Head: InsertColumn<Table>,
    Head::Column: SQLColumnInfo + Default,
    Tail: InsertTargetColumnList<Table>,
{
    type Columns = Cons<Head::Column, Tail::Columns>;

    fn append_columns<'a, V: SQLParam>(sql: &mut SQL<'a, V>) {
        let name = Head::Column::default().name();
        assert!(
            !sql.chunks.iter().any(
                |chunk| matches!(chunk, SQLChunk::Ident(identifier) if identifier.as_ref() == name),
            ),
            "an INSERT target column cannot appear more than once",
        );
        if !sql.chunks.is_empty() {
            sql.push_mut(Token::COMMA);
        }
        sql.append_mut(SQL::ident(name));
        Tail::append_columns(sql);
    }
}

impl<'a, V, Table, Columns> InsertTargetColumns<'a, V, Table> for Columns
where
    V: SQLParam,
    Columns: IntoSelectTarget,
    Columns::Marker: InsertTargetMarker<Table>,
    <Columns::Marker as InsertTargetMarker<Table>>::Columns: InsertTargetColumnList<Table>,
{
    type Columns = <Columns::Marker as InsertTargetMarker<Table>>::Columns;

    fn into_target_columns_sql(self) -> SQL<'a, V> {
        let mut columns = SQL::empty();
        <<Columns::Marker as InsertTargetMarker<Table>>::Columns as InsertTargetColumnList<
            Table,
        >>::append_columns(&mut columns);
        assert!(
            !columns.chunks.is_empty(),
            "an INSERT target list must contain at least one column",
        );
        columns.parens()
    }
}

/// Proof that an explicit target list contains every database-required column.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "the INSERT target list omits one or more required columns",
    label = "add every non-null column without a database default to `.columns(...)`"
)]
pub trait IncludesRequired<Required, Proof> {}

impl<Targets, Required, Proof> IncludesRequired<Required, Proof> for Targets where
    Targets: ListIncludes<Required, Proof>
{
}

/// A checked SELECT projection for an explicit INSERT target list.
#[doc(hidden)]
pub trait PartialInsertSelectCompatible<'a, V: SQLParam, Targets> {}

impl<'a, V, M, Scope, Used, Targets> PartialInsertSelectCompatible<'a, V, Targets>
    for Scoped<M, Scope, Used>
where
    V: SQLParam,
    M: PartialInsertSelectCompatible<'a, V, Targets>,
{
}

impl<'a, V, Targets, Selected> PartialInsertSelectCompatible<'a, V, Targets>
    for SelectCols<Selected>
where
    V: SQLParam,
    Selected: SelectedExpressionList,
    Targets: InsertSelectColumns<'a, V, Selected::Expressions>,
{
}

/// A checked SELECT projection for every insertable target column.
#[diagnostic::on_unimplemented(
    message = "this query cannot be the source of an INSERT into `{Target}`",
    label = "the source must be a SELECT of the insertable columns",
    note = "an UPDATE or DELETE, even with RETURNING, cannot feed an INSERT"
)]
#[doc(hidden)]
pub trait InsertSelectCompatible<'a, V: SQLParam, Target, Row> {}

impl<'a, V, M, Scope, Used, Target, Row> InsertSelectCompatible<'a, V, Target, Row>
    for Scoped<M, Scope, Used>
where
    V: SQLParam,
    M: InsertSelectCompatible<'a, V, Target, Row>,
{
}

impl<'a, V, Target, Row> InsertSelectCompatible<'a, V, Target, Row> for SelectStar
where
    V: SQLParam,
    Target: HasSelectModel + InsertSelectAllColumns,
    Row: TypeEq<Target::SelectModel>,
{
}

impl<'a, V, Target, Row, Selected> InsertSelectCompatible<'a, V, Target, Row>
    for SelectCols<Selected>
where
    V: SQLParam,
    Target: InsertSelectTable,
    Selected: SelectedExpressionList,
    Target::Columns: InsertSelectColumns<'a, V, Selected::Expressions>,
{
}

impl<'a, V, Target, Row, Selected> InsertSelectCompatible<'a, V, Target, Row> for SelectAs<Selected>
where
    V: SQLParam,
    Target: HasSelectModel + InsertSelectAllColumns,
    Row: TypeEq<Target::SelectModel>,
{
}
