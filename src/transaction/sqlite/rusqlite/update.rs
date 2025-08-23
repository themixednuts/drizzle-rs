use std::marker::PhantomData;

use drizzle_core::SQLTable;
use drizzle_sqlite::{
    SQLiteValue,
    builder::{UpdateInitial, UpdateSetClauseSet, UpdateWhereSet, update::UpdateBuilder},
};

use crate::transaction::sqlite::rusqlite::TransactionBuilder;

impl<'a, 'conn, Schema, Table>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        UpdateBuilder<'a, Schema, UpdateInitial, Table>,
        UpdateInitial,
    >
where
    Table: SQLTable<'a, SQLiteValue<'a>>,
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
        condition: drizzle_core::SQL<'a, SQLiteValue<'a>>,
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
}
