use crate::transaction::postgres::postgres_sync::TransactionBuilder;
use drizzle_core::{ConflictTarget, NamedConstraint, SQLModel, ToSQL};
use drizzle_postgres::builder::{
    InsertDoUpdateSet, InsertInitial, InsertOnConflictSet, InsertReturningSet, InsertValuesSet,
    OnConflictBuilder, insert::InsertBuilder,
};
use drizzle_postgres::traits::PostgresTable;
use drizzle_postgres::values::PostgresValue;
use std::marker::PhantomData;

type ReturningMarker<Table, Columns> = drizzle_core::Scoped<
    <Columns as drizzle_core::IntoSelectTarget>::Marker,
    drizzle_core::Cons<Table, drizzle_core::Nil>,
>;

type ReturningRow<Table, Columns> =
    <<Columns as drizzle_core::IntoSelectTarget>::Marker as drizzle_core::ResolveRow<Table>>::Row;

type InsertReturningTxBuilder<'tx, 'conn, 'q, Schema, Table, Columns> = TransactionBuilder<
    'tx,
    'conn,
    Schema,
    InsertBuilder<
        'q,
        Schema,
        InsertReturningSet,
        Table,
        ReturningMarker<Table, Columns>,
        ReturningRow<Table, Columns>,
    >,
    InsertReturningSet,
>;

/// Intermediate builder for typed ON CONFLICT within a postgres-sync transaction.
pub struct TransactionOnConflictBuilder<'tx, 'conn, 'q, Schema, Table> {
    transaction: &'tx super::Transaction<'conn, Schema>,
    builder: OnConflictBuilder<'q, Schema, Table>,
}

impl<'tx, 'conn, 'q, Schema, Table> TransactionOnConflictBuilder<'tx, 'conn, 'q, Schema, Table> {
    /// Adds a WHERE clause to the conflict target for partial index matching.
    pub fn r#where<E>(mut self, condition: E) -> Self
    where
        E: drizzle_core::expr::Expr<'q, PostgresValue<'q>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        self.builder = self.builder.r#where(condition);
        self
    }

    /// `ON CONFLICT (cols) DO NOTHING`
    pub fn do_nothing(
        self,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        InsertBuilder<'q, Schema, InsertOnConflictSet, Table>,
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
        set: impl ToSQL<'q, PostgresValue<'q>>,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        InsertBuilder<'q, Schema, InsertDoUpdateSet, Table>,
        InsertDoUpdateSet,
    > {
        TransactionBuilder {
            transaction: self.transaction,
            builder: self.builder.do_update(set),
            _phantom: PhantomData,
        }
    }
}

impl<'tx, 'conn, 'q, Schema, Table>
    TransactionBuilder<
        'tx,
        'conn,
        Schema,
        InsertBuilder<'q, Schema, InsertInitial, Table>,
        InsertInitial,
    >
{
    #[inline]
    pub fn value<T>(
        self,
        value: Table::Insert<T>,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        InsertBuilder<'q, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
    where
        Table: PostgresTable<'q>,
        Table::Insert<T>: SQLModel<'q, PostgresValue<'q>>,
    {
        self.values([value])
    }

    #[inline]
    pub fn values<T>(
        self,
        values: impl IntoIterator<Item = Table::Insert<T>>,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        InsertBuilder<'q, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
    where
        Table: PostgresTable<'q>,
        Table::Insert<T>: SQLModel<'q, PostgresValue<'q>>,
    {
        let builder = self.builder.values(values);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

impl<'tx, 'conn, 'q, Schema, Table>
    TransactionBuilder<
        'tx,
        'conn,
        Schema,
        InsertBuilder<'q, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
where
    Table: PostgresTable<'q>,
{
    /// Begins a typed ON CONFLICT clause targeting specific columns.
    pub fn on_conflict<C: ConflictTarget<Table>>(
        self,
        target: C,
    ) -> TransactionOnConflictBuilder<'tx, 'conn, 'q, Schema, Table> {
        TransactionOnConflictBuilder {
            transaction: self.transaction,
            builder: self.builder.on_conflict(target),
        }
    }

    /// Begins a typed ON CONFLICT ON CONSTRAINT clause (PostgreSQL-only).
    pub fn on_conflict_on_constraint<C: NamedConstraint<Table>>(
        self,
        target: C,
    ) -> TransactionOnConflictBuilder<'tx, 'conn, 'q, Schema, Table> {
        TransactionOnConflictBuilder {
            transaction: self.transaction,
            builder: self.builder.on_conflict_on_constraint(target),
        }
    }

    /// Shorthand for `ON CONFLICT DO NOTHING` without specifying a target.
    pub fn on_conflict_do_nothing(
        self,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        InsertBuilder<'q, Schema, InsertOnConflictSet, Table>,
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
    ) -> InsertReturningTxBuilder<'tx, 'conn, 'q, Schema, Table, Columns>
    where
        Columns: ToSQL<'q, PostgresValue<'q>> + drizzle_core::IntoSelectTarget,
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

impl<'tx, 'conn, 'q, Schema, Table>
    TransactionBuilder<
        'tx,
        'conn,
        Schema,
        InsertBuilder<'q, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    >
{
    /// Adds RETURNING clause after ON CONFLICT
    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> InsertReturningTxBuilder<'tx, 'conn, 'q, Schema, Table, Columns>
    where
        Columns: ToSQL<'q, PostgresValue<'q>> + drizzle_core::IntoSelectTarget,
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

impl<'tx, 'conn, 'q, Schema, Table>
    TransactionBuilder<
        'tx,
        'conn,
        Schema,
        InsertBuilder<'q, Schema, InsertDoUpdateSet, Table>,
        InsertDoUpdateSet,
    >
{
    /// Adds WHERE clause after DO UPDATE SET
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        InsertBuilder<'q, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    >
    where
        E: drizzle_core::expr::Expr<'q, PostgresValue<'q>>,
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
    ) -> InsertReturningTxBuilder<'tx, 'conn, 'q, Schema, Table, Columns>
    where
        Columns: ToSQL<'q, PostgresValue<'q>> + drizzle_core::IntoSelectTarget,
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
