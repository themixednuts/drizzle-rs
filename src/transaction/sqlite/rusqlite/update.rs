use drizzle_sqlite::builder::{
    UpdateInitial, UpdateSetClauseSet, UpdateWhereSet, update::UpdateBuilder,
};
use drizzle_sqlite::traits::SQLiteTable;
use drizzle_sqlite::values::SQLiteValue;
use std::marker::PhantomData;

use crate::transaction::sqlite::rusqlite::TransactionBuilder;

impl<'tx, 'a, 'conn, Schema, Table>
    TransactionBuilder<
        'tx,
        'conn,
        Schema,
        UpdateBuilder<'a, Schema, UpdateInitial, Table>,
        UpdateInitial,
    >
where
    Table: SQLiteTable<'a>,
{
    #[inline]
    pub fn set(
        self,
        values: Table::Update,
    ) -> TransactionBuilder<
        'tx,
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

impl<'tx, 'a, 'conn, Schema, Table>
    TransactionBuilder<
        'tx,
        'conn,
        Schema,
        UpdateBuilder<'a, Schema, UpdateSetClauseSet, Table>,
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
        UpdateBuilder<'a, Schema, UpdateWhereSet, Table>,
        UpdateWhereSet,
    >
    where
        E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        let builder = self.builder.r#where(condition);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}
