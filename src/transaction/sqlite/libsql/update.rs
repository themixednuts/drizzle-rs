use std::marker::PhantomData;

use drizzle_sqlite::builder::{
    UpdateInitial, UpdateSetClauseSet, UpdateWhereSet, update::UpdateBuilder,
};
use drizzle_sqlite::traits::SQLiteTable;
use drizzle_sqlite::values::SQLiteValue;

use crate::transaction::sqlite::libsql::TransactionBuilder;

impl<'tx, 'a, Schema, Table>
    TransactionBuilder<'tx, Schema, UpdateBuilder<'a, Schema, UpdateInitial, Table>, UpdateInitial>
where
    Table: SQLiteTable<'a>,
{
    #[inline]
    pub fn set(
        self,
        values: Table::Update,
    ) -> TransactionBuilder<
        'tx,
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

impl<'tx, 'a, Schema, Table>
    TransactionBuilder<
        'tx,
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
