use crate::transaction::sqlite::turso::TransactionBuilder;
use drizzle_sqlite::{
    builder::{DeleteInitial, DeleteWhereSet, delete::DeleteBuilder},
    traits::SQLiteTable,
    values::SQLiteValue,
};
use std::marker::PhantomData;

impl<'tx, 'a, 'conn, S, T>
    TransactionBuilder<'tx, 'conn, S, DeleteBuilder<'a, S, DeleteInitial, T>, DeleteInitial>
where
    T: SQLiteTable<'a>,
{
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> TransactionBuilder<'tx, 'conn, S, DeleteBuilder<'a, S, DeleteWhereSet, T>, DeleteWhereSet>
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
