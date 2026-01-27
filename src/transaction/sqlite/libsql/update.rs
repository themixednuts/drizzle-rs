use std::marker::PhantomData;

use drizzle_sqlite::builder::{
    UpdateInitial, UpdateSetClauseSet, UpdateWhereSet, update::UpdateBuilder,
};
use drizzle_sqlite::traits::SQLiteTable;
use drizzle_sqlite::values::SQLiteValue;

use crate::transaction::sqlite::libsql::TransactionBuilder;

impl<'a, Schema, Table>
    TransactionBuilder<'a, Schema, UpdateBuilder<'a, Schema, UpdateInitial, Table>, UpdateInitial>
where
    Table: SQLiteTable<'a>,
{
    #[inline]
    pub fn set(
        self,
        values: Table::Update,
    ) -> TransactionBuilder<
        'a,
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

impl<'a, Schema, Table>
    TransactionBuilder<
        'a,
        Schema,
        UpdateBuilder<'a, Schema, UpdateSetClauseSet, Table>,
        UpdateSetClauseSet,
    >
{
    pub fn r#where(
        self,
        condition: impl drizzle_core::traits::ToSQL<'a, SQLiteValue<'a>>,
    ) -> TransactionBuilder<
        'a,
        Schema,
        UpdateBuilder<'a, Schema, UpdateWhereSet, Table>,
        UpdateWhereSet,
    > {
        let builder = self.builder.r#where(condition.to_sql());
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}
