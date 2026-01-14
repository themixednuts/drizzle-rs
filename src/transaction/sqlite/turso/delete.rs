use crate::transaction::sqlite::turso::TransactionBuilder;
use drizzle_sqlite::{
    SQLiteValue,
    builder::{DeleteInitial, DeleteWhereSet, delete::DeleteBuilder},
    traits::SQLiteTable,
};
use std::marker::PhantomData;

impl<'a, 'conn, S, T>
    TransactionBuilder<'a, 'conn, S, DeleteBuilder<'a, S, DeleteInitial, T>, DeleteInitial>
where
    T: SQLiteTable<'a>,
{
    pub fn r#where(
        self,
        condition: impl drizzle_core::traits::ToSQL<'a, SQLiteValue<'a>>,
    ) -> TransactionBuilder<'a, 'conn, S, DeleteBuilder<'a, S, DeleteWhereSet, T>, DeleteWhereSet>
    {
        let builder = self.builder.r#where(condition.to_sql());
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}
