use crate::transaction::sqlite::turso::TransactionBuilder;
use drizzle_core::{ConflictTarget, SQLModel, ToSQL};
use drizzle_sqlite::{
    builder::{
        InsertDoUpdateSet, InsertInitial, InsertOnConflictSet, InsertReturningSet, InsertValuesSet,
        OnConflictBuilder, insert::InsertBuilder,
    },
    traits::SQLiteTable,
    values::SQLiteValue,
};
use std::marker::PhantomData;

type ReturningMarker<Table, Columns> = drizzle_core::Scoped<
    <Columns as drizzle_core::IntoSelectTarget>::Marker,
    drizzle_core::Cons<Table, drizzle_core::Nil>,
>;

type ReturningRow<Table, Columns> =
    <<Columns as drizzle_core::IntoSelectTarget>::Marker as drizzle_core::ResolveRow<Table>>::Row;

type InsertReturningTxBuilder<'a, 'conn, Schema, Table, Columns> = TransactionBuilder<
    'a,
    'conn,
    Schema,
    InsertBuilder<
        'a,
        Schema,
        InsertReturningSet,
        Table,
        ReturningMarker<Table, Columns>,
        ReturningRow<Table, Columns>,
    >,
    InsertReturningSet,
>;

/// Intermediate builder for typed ON CONFLICT within a turso transaction.
pub struct TransactionOnConflictBuilder<'a, 'conn, Schema, Table> {
    transaction: &'a super::Transaction<'conn, Schema>,
    builder: OnConflictBuilder<'a, Schema, Table>,
}

impl<'a, 'conn, Schema, Table> TransactionOnConflictBuilder<'a, 'conn, Schema, Table> {
    /// Adds a WHERE clause to the conflict target for partial index matching.
    pub fn r#where<E>(mut self, condition: E) -> Self
    where
        E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        self.builder = self.builder.r#where(condition);
        self
    }

    /// `ON CONFLICT (cols) DO NOTHING`
    pub fn do_nothing(
        self,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    > {
        TransactionBuilder {
            transaction: self.transaction,
            builder: self.builder.do_nothing(),
            _phantom: PhantomData,
        }
    }

    /// `ON CONFLICT (cols) DO UPDATE SET ...`
    pub fn do_update(
        self,
        set: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertDoUpdateSet, Table>,
        InsertDoUpdateSet,
    > {
        TransactionBuilder {
            transaction: self.transaction,
            builder: self.builder.do_update(set),
            _phantom: PhantomData,
        }
    }
}

impl<'a, 'conn, Schema, Table>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertInitial, Table>,
        InsertInitial,
    >
{
    #[inline]
    pub fn value<T>(
        self,
        value: Table::Insert<T>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
    where
        Table: SQLiteTable<'a>,
        Table::Insert<T>: SQLModel<'a, SQLiteValue<'a>>,
    {
        self.values([value])
    }

    #[inline]
    pub fn values<T>(
        self,
        values: impl IntoIterator<Item = Table::Insert<T>>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
    where
        Table: SQLiteTable<'a>,
        Table::Insert<T>: SQLModel<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.values(values);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

impl<'a, 'conn, Schema, Table>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
where
    Table: SQLiteTable<'a>,
{
    /// Begins a typed ON CONFLICT clause targeting a specific constraint.
    pub fn on_conflict<C: ConflictTarget<Table>>(
        self,
        target: C,
    ) -> TransactionOnConflictBuilder<'a, 'conn, Schema, Table> {
        TransactionOnConflictBuilder {
            transaction: self.transaction,
            builder: self.builder.on_conflict(target),
        }
    }

    /// Shorthand for `ON CONFLICT DO NOTHING` without specifying a target.
    pub fn on_conflict_do_nothing(
        self,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    > {
        TransactionBuilder {
            transaction: self.transaction,
            builder: self.builder.on_conflict_do_nothing(),
            _phantom: PhantomData,
        }
    }

    /// Adds RETURNING clause
    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> InsertReturningTxBuilder<'a, 'conn, Schema, Table, Columns>
    where
        Columns: ToSQL<'a, SQLiteValue<'a>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<Table>,
    {
        let builder = self.builder.returning(columns);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

impl<'a, 'conn, Schema, Table>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    >
{
    /// Adds RETURNING clause after ON CONFLICT
    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> InsertReturningTxBuilder<'a, 'conn, Schema, Table, Columns>
    where
        Columns: ToSQL<'a, SQLiteValue<'a>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<Table>,
    {
        let builder = self.builder.returning(columns);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

impl<'a, 'conn, Schema, Table>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertDoUpdateSet, Table>,
        InsertDoUpdateSet,
    >
{
    /// Adds WHERE clause after DO UPDATE SET
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    >
    where
        E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        TransactionBuilder {
            transaction: self.transaction,
            builder: self.builder.r#where(condition),
            _phantom: PhantomData,
        }
    }

    /// Adds RETURNING clause after DO UPDATE SET
    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> InsertReturningTxBuilder<'a, 'conn, Schema, Table, Columns>
    where
        Columns: ToSQL<'a, SQLiteValue<'a>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<Table>,
    {
        let builder = self.builder.returning(columns);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}
