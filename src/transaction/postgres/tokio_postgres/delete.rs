use crate::transaction::postgres::tokio_postgres::TransactionBuilder;
use drizzle_core::ToSQL;
use drizzle_postgres::builder::{
    DeleteInitial, DeleteReturningSet, DeleteWhereSet, delete::DeleteBuilder,
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

type DeleteReturningTxBuilder<'tx, 'conn, 'q, S, T, Columns> = TransactionBuilder<
    'tx,
    'conn,
    S,
    DeleteBuilder<
        'q,
        S,
        DeleteReturningSet,
        T,
        ReturningMarker<T, Columns>,
        ReturningRow<T, Columns>,
    >,
    DeleteReturningSet,
>;

impl<'tx, 'conn, 'q, S, T>
    TransactionBuilder<'tx, 'conn, S, DeleteBuilder<'q, S, DeleteInitial, T>, DeleteInitial>
where
    T: PostgresTable<'q>,
{
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> TransactionBuilder<'tx, 'conn, S, DeleteBuilder<'q, S, DeleteWhereSet, T>, DeleteWhereSet>
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
    ) -> DeleteReturningTxBuilder<'tx, 'conn, 'q, S, T, Columns>
    where
        Columns: ToSQL<'q, PostgresValue<'q>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<T>,
    {
        let builder = self.builder.returning(columns);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

impl<'tx, 'conn, 'q, S, T>
    TransactionBuilder<'tx, 'conn, S, DeleteBuilder<'q, S, DeleteWhereSet, T>, DeleteWhereSet>
{
    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> DeleteReturningTxBuilder<'tx, 'conn, 'q, S, T, Columns>
    where
        Columns: ToSQL<'q, PostgresValue<'q>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<T>,
    {
        let builder = self.builder.returning(columns);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}
