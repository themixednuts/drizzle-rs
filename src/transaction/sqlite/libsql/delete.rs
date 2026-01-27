use crate::transaction::sqlite::libsql::TransactionBuilder;
use drizzle_sqlite::builder::{DeleteInitial, DeleteWhereSet, delete::DeleteBuilder};
use drizzle_sqlite::traits::SQLiteTable;
use drizzle_sqlite::values::SQLiteValue;
use std::marker::PhantomData;

impl<'a, S, T> TransactionBuilder<'a, S, DeleteBuilder<'a, S, DeleteInitial, T>, DeleteInitial>
where
    T: SQLiteTable<'a>,
{
    pub fn r#where(
        self,
        condition: impl drizzle_core::traits::ToSQL<'a, SQLiteValue<'a>>,
    ) -> TransactionBuilder<'a, S, DeleteBuilder<'a, S, DeleteWhereSet, T>, DeleteWhereSet> {
        let builder = self.builder.r#where(condition.to_sql());
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}
