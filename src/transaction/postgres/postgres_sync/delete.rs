use crate::transaction::postgres::postgres_sync::TransactionBuilder;
use drizzle_core::ToSQL;
use drizzle_postgres::{
    PostgresValue,
    builder::{DeleteInitial, DeleteReturningSet, DeleteWhereSet, delete::DeleteBuilder},
    traits::PostgresTable,
};
use std::marker::PhantomData;

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

    pub fn returning(
        self,
        columns: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        S,
        DeleteBuilder<'a, S, DeleteReturningSet, T>,
        DeleteReturningSet,
    > {
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
    pub fn returning(
        self,
        columns: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        S,
        DeleteBuilder<'a, S, DeleteReturningSet, T>,
        DeleteReturningSet,
    > {
        let builder = self.builder.returning(columns);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}
