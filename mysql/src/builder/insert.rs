use super::select::IntoSelect;
use crate::{
    traits::{MySQLInsertSelectTarget, MySQLTable},
    values::MySQLValue,
};

pub use drizzle_core::builder::{InsertInitial, InsertValuesSet};

/// Marker for `INSERT IGNORE` before its row source is supplied.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertIgnoreSet;

/// Marker for a completed `ON DUPLICATE KEY UPDATE` insert.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertOnDuplicateKeyUpdateSet;

impl drizzle_core::ExecutableState for InsertOnDuplicateKeyUpdateSet {}

pub type InsertBuilder<'a, Schema, State, Table, Marker = (), Row = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row>;

/// Pairwise compatibility between target columns and SELECT expressions.
#[doc(hidden)]
pub trait InsertSelectColumns<'a, Source> {}

impl<'a> InsertSelectColumns<'a, drizzle_core::Nil> for drizzle_core::Nil {}

impl<'a, TargetExpr, TargetTail, SourceExpr, SourceTail>
    InsertSelectColumns<'a, drizzle_core::Cons<SourceExpr, SourceTail>>
    for drizzle_core::Cons<TargetExpr, TargetTail>
where
    TargetExpr: drizzle_core::expr::Expr<'a, MySQLValue<'a>>,
    SourceExpr: drizzle_core::expr::Expr<'a, MySQLValue<'a>>,
    TargetExpr::SQLType: drizzle_core::types::Assignable<SourceExpr::SQLType>,
    TargetExpr::Nullable:
        drizzle_core::expr::NullAnd<SourceExpr::Nullable, Output = SourceExpr::Nullable>,
    TargetTail: InsertSelectColumns<'a, SourceTail>,
{
}

/// A completed SELECT projection that can fill every target table column in
/// declaration order.
#[doc(hidden)]
pub trait InsertSelectCompatible<'a, Target, Row> {}

impl<'a, M, Scope, Target, Row> InsertSelectCompatible<'a, Target, Row>
    for drizzle_core::Scoped<M, Scope>
where
    M: InsertSelectCompatible<'a, Target, Row>,
{
}

impl<'a, Target, Row> InsertSelectCompatible<'a, Target, Row> for drizzle_core::SelectStar
where
    Target: drizzle_core::HasSelectModel,
    Row: drizzle_core::TypeEq<<Target as drizzle_core::HasSelectModel>::SelectModel>,
{
}

impl<'a, Target, Row, Specified> InsertSelectCompatible<'a, Target, Row>
    for drizzle_core::SelectAs<Specified>
where
    Target: drizzle_core::HasSelectModel,
    Row: drizzle_core::TypeEq<<Target as drizzle_core::HasSelectModel>::SelectModel>,
{
}

impl<'a, Target, Row, Cols> InsertSelectCompatible<'a, Target, Row>
    for drizzle_core::SelectCols<Cols>
where
    Target: MySQLInsertSelectTarget,
    Cols: drizzle_core::SelectedExpressionList,
    Target::Columns:
        InsertSelectColumns<'a, <Cols as drizzle_core::SelectedExpressionList>::Expressions>,
{
}

impl<'a, S, T, M, R> InsertBuilder<'a, S, InsertValuesSet, T, M, R> {
    /// Compiles named or anonymous placeholders into MySQL's ordered
    /// positional bind plan.
    #[must_use]
    pub fn prepare(&self) -> drizzle_core::prepared::PreparedStatement<'a, MySQLValue<'a>> {
        self.prepared_statement()
    }

    /// Updates columns when any primary or unique key conflicts.
    ///
    /// MySQL chooses the conflicting key. There is deliberately no conflict
    /// target parameter because MySQL cannot express one in this clause.
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
    /// Compiles this upsert into MySQL's ordered positional bind plan.
    #[must_use]
    pub fn prepare(&self) -> drizzle_core::prepared::PreparedStatement<'a, MySQLValue<'a>> {
        self.prepared_statement()
    }
}

impl<'a, Schema, Table> InsertBuilder<'a, Schema, InsertInitial, Table>
where
    Table: MySQLTable<'a>,
{
    /// Changes this statement to `INSERT IGNORE`.
    ///
    /// This is MySQL's broad warning/error suppression form. It is not a
    /// target-bearing conflict clause and should only be used when those
    /// wider semantics are intended.
    #[must_use]
    pub fn ignore(self) -> InsertBuilder<'a, Schema, InsertIgnoreSet, Table> {
        InsertBuilder::from_sql(crate::helpers::insert_ignore(self.sql))
    }
}

/// Insert states that can still receive a VALUES or SELECT row source.
#[doc(hidden)]
pub trait InsertRowSourceState {}

impl InsertRowSourceState for InsertInitial {}
impl InsertRowSourceState for InsertIgnoreSet {}

impl<'a, Schema, State, Table> InsertBuilder<'a, Schema, State, Table>
where
    State: InsertRowSourceState,
    Table: MySQLTable<'a>,
{
    pub fn value<T>(
        self,
        value: Table::Insert<T>,
    ) -> InsertBuilder<'a, Schema, InsertValuesSet, Table> {
        self.values([value])
    }

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

    /// Inserts the result of a SELECT. A WITH clause, when present, belongs to
    /// the SELECT and therefore renders after the INSERT target in MySQL.
    pub fn select<Q, R>(self, query: Q) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        Table: MySQLInsertSelectTarget,
        Q: super::select::IntoSelectQuery<'a, Schema, R>,
        Q::Marker: InsertSelectCompatible<'a, Table, R>,
    {
        InsertBuilder::from_sql(self.sql.append(query.into_select_query().into_select_sql()))
    }
}
