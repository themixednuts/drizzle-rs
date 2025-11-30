use drizzle_postgres::{
    PostgresValue,
    builder::{UpdateInitial, UpdateReturningSet, UpdateSetClauseSet, UpdateWhereSet, update::UpdateBuilder},
    traits::PostgresTable,
};
use drizzle_core::ToSQL;
use std::marker::PhantomData;

use crate::transaction::postgres::postgres_sync::TransactionBuilder;

impl<'a, 'conn, Schema, Table>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        UpdateBuilder<'a, Schema, UpdateInitial, Table>,
        UpdateInitial,
    >
where
    Table: PostgresTable<'a>,
{
    #[inline]
    pub fn set(
        self,
        values: Table::Update,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        UpdateBuilder<'a, Schema, UpdateSetClauseSet, Table>,
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

impl<'a, 'conn, Schema, Table>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        UpdateBuilder<'a, Schema, UpdateSetClauseSet, Table>,
        UpdateSetClauseSet,
    >
{
    pub fn r#where(
        self,
        condition: drizzle_core::SQL<'a, PostgresValue<'a>>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        UpdateBuilder<'a, Schema, UpdateWhereSet, Table>,
        UpdateWhereSet,
    > {
        let builder = self.builder.r#where(condition);
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
        Schema,
        UpdateBuilder<'a, Schema, UpdateReturningSet, Table>,
        UpdateReturningSet,
    > {
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
        UpdateBuilder<'a, Schema, UpdateWhereSet, Table>,
        UpdateWhereSet,
    >
{
    pub fn returning(
        self,
        columns: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        UpdateBuilder<'a, Schema, UpdateReturningSet, Table>,
        UpdateReturningSet,
    > {
        let builder = self.builder.returning(columns);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}
