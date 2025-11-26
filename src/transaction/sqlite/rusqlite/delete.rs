use crate::transaction::sqlite::rusqlite::TransactionBuilder;
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
        condition: drizzle_core::SQL<'a, SQLiteValue<'a>>,
    ) -> TransactionBuilder<'a, 'conn, S, DeleteBuilder<'a, S, DeleteWhereSet, T>, DeleteWhereSet>
    {
        let builder = self.builder.r#where(condition);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}
