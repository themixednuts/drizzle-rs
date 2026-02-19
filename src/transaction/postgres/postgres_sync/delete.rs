use crate::transaction::postgres::postgres_sync::TransactionBuilder;
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

type DeleteReturningTxBuilder<'a, 'conn, S, T, Columns> = TransactionBuilder<
    'a,
    'conn,
    S,
    DeleteBuilder<
        'a,
        S,
        DeleteReturningSet,
        T,
        ReturningMarker<T, Columns>,
        ReturningRow<T, Columns>,
    >,
    DeleteReturningSet,
>;

impl<'a, 'conn, S, T>
    TransactionBuilder<'a, 'conn, S, DeleteBuilder<'a, S, DeleteInitial, T>, DeleteInitial>
where
    T: PostgresTable<'a>,
{
    pub fn r#where(
        self,
        condition: impl drizzle_core::traits::ToSQL<'a, PostgresValue<'a>>,
    ) -> TransactionBuilder<'a, 'conn, S, DeleteBuilder<'a, S, DeleteWhereSet, T>, DeleteWhereSet>
    {
        let builder = self.builder.r#where(condition.to_sql());
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> DeleteReturningTxBuilder<'a, 'conn, S, T, Columns>
    where
        Columns: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
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

impl<'a, 'conn, S, T>
    TransactionBuilder<'a, 'conn, S, DeleteBuilder<'a, S, DeleteWhereSet, T>, DeleteWhereSet>
{
    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> DeleteReturningTxBuilder<'a, 'conn, S, T, Columns>
    where
        Columns: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
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
