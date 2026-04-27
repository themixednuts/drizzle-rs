use drizzle_core::ToSQL;
use drizzle_postgres::builder::{
    UpdateInitial, UpdateReturningSet, UpdateSetClauseSet, UpdateWhereSet, update::UpdateBuilder,
};
use drizzle_postgres::traits::PostgresTable;
use drizzle_postgres::values::PostgresValue;
use std::marker::PhantomData;

use crate::transaction::postgres::postgres_sync::TransactionBuilder;

type ReturningMarker<Table, Columns> = drizzle_core::Scoped<
    <Columns as drizzle_core::IntoSelectTarget>::Marker,
    drizzle_core::Cons<Table, drizzle_core::Nil>,
>;

type ReturningRow<Table, Columns> =
    <<Columns as drizzle_core::IntoSelectTarget>::Marker as drizzle_core::ResolveRow<Table>>::Row;

type UpdateReturningTxBuilder<'tx, 'conn, 'q, Schema, Table, Columns> = TransactionBuilder<
    'tx,
    'conn,
    Schema,
    UpdateBuilder<
        'q,
        Schema,
        UpdateReturningSet,
        Table,
        ReturningMarker<Table, Columns>,
        ReturningRow<Table, Columns>,
    >,
    UpdateReturningSet,
>;

impl<'tx, 'conn, 'q, Schema, Table>
    TransactionBuilder<
        'tx,
        'conn,
        Schema,
        UpdateBuilder<'q, Schema, UpdateInitial, Table>,
        UpdateInitial,
    >
where
    Table: PostgresTable<'q>,
{
    #[inline]
    pub fn set(
        self,
        values: Table::Update,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        UpdateBuilder<'q, Schema, UpdateSetClauseSet, Table>,
        UpdateSetClauseSet,
    > {
        let builder = self.builder.set(values);
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
        UpdateBuilder<'q, Schema, UpdateSetClauseSet, Table>,
        UpdateSetClauseSet,
    >
{
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> TransactionBuilder<
        'tx,
        'conn,
        Schema,
        UpdateBuilder<'q, Schema, UpdateWhereSet, Table>,
        UpdateWhereSet,
    >
    where
        E: drizzle_core::expr::Expr<'q, PostgresValue<'q>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        let builder = self.builder.r#where(condition);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> UpdateReturningTxBuilder<'tx, 'conn, 'q, Schema, Table, Columns>
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
        UpdateBuilder<'q, Schema, UpdateWhereSet, Table>,
        UpdateWhereSet,
    >
{
    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> UpdateReturningTxBuilder<'tx, 'conn, 'q, Schema, Table, Columns>
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
