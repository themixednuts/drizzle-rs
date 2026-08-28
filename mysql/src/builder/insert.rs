use super::select::{CompletedSelect, IntoSelectQuery};
use crate::{traits::MySQLTable, values::MySQLValue};
use drizzle_core::{
    IncludesRequired, InsertSelectCompatible, InsertSelectTable, InsertTargetColumns,
    PartialInsertSelectCompatible, ToSQL,
};

pub use drizzle_core::builder::{InsertColumnsSet, InsertInitial, InsertValuesSet};

/// Marker for `INSERT IGNORE` before its row source is supplied.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertIgnoreSet;

/// Marker for a completed `ON DUPLICATE KEY UPDATE` insert.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertOnDuplicateKeyUpdateSet;

impl drizzle_core::ExecutableState for InsertOnDuplicateKeyUpdateSet {}

/// Typed MySQL `INSERT` builder.
pub type InsertBuilder<'a, Schema, State, Table, Marker = (), Row = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row>;

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
    /// Chooses an explicit ordered target-column list for an INSERT SELECT.
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

    /// Adds one row to this insert.
    pub fn value<T>(
        self,
        value: Table::Insert<T>,
    ) -> InsertBuilder<'a, Schema, InsertValuesSet, Table> {
        self.values([value])
    }

    /// Adds multiple rows to this insert.
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
    pub fn select<Q, R, ScopeProof, AggProof>(
        self,
        query: Q,
    ) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        Table: InsertSelectTable,
        Q: IntoSelectQuery<'a, Schema, R>,
        Q::Marker: InsertSelectCompatible<'a, MySQLValue<'a>, Table, R>
            + drizzle_core::InsertSourceInScope<ScopeProof>
            + drizzle_core::MarkerAggValidFor<Q::Grouped, AggProof>,
    {
        InsertBuilder::from_sql(
            self.sql
                .append(Table::insert_columns_sql::<MySQLValue<'a>>())
                .append(query.into_select_query().into_select_sql()),
        )
    }

    /// Inserts an unchecked raw SELECT without an explicit target list.
    ///
    /// This opts out of projection shape, type, nullability, source-scope, and
    /// aggregate validation.
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
    /// Inserts an explicit SELECT projection into the chosen target columns.
    pub fn select<Q, R, RequiredProof, ScopeProof, AggProof>(
        self,
        query: Q,
    ) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        Targets: IncludesRequired<Table::RequiredColumns, RequiredProof>,
        Table: InsertSelectTable,
        Q: IntoSelectQuery<'a, Schema, R>,
        Q::Marker: PartialInsertSelectCompatible<'a, MySQLValue<'a>, Targets>
            + drizzle_core::InsertSourceInScope<ScopeProof>
            + drizzle_core::MarkerAggValidFor<Q::Grouped, AggProof>,
    {
        InsertBuilder::from_sql(self.sql.append(query.into_select_query().into_select_sql()))
    }

    /// Inserts an unchecked raw SELECT into the chosen target columns.
    ///
    /// This opts out of projection shape, type, nullability, source-scope, and
    /// aggregate validation.
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
