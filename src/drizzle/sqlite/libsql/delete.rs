use crate::drizzle::sqlite::libsql::DrizzleBuilder;
use drizzle_sqlite::{
    SQLiteValue,
    builder::{DeleteInitial, DeleteWhereSet, delete::DeleteBuilder},
    traits::SQLiteTable,
};
use std::marker::PhantomData;

impl<'a, S, T> DrizzleBuilder<'a, S, DeleteBuilder<'a, S, DeleteInitial, T>, DeleteInitial>
where
    T: SQLiteTable<'a>,
{
    pub fn r#where(
        self,
        condition: drizzle_core::SQL<'a, SQLiteValue<'a>>,
    ) -> DrizzleBuilder<'a, S, DeleteBuilder<'a, S, DeleteWhereSet, T>, DeleteWhereSet> {
        let builder = self.builder.r#where(condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}
