use crate::drizzle::sqlite::turso::DrizzleBuilder;
use drizzle_sqlite::{
    SQLiteValue,
    builder::{DeleteInitial, DeleteWhereSet, delete::DeleteBuilder},
    traits::SQLiteTable,
};
use std::marker::PhantomData;

impl<'a, 'b, S, T> DrizzleBuilder<'a, S, DeleteBuilder<'b, S, DeleteInitial, T>, DeleteInitial>
where
    T: SQLiteTable<'b>,
{
    pub fn r#where(
        self,
        condition: drizzle_core::SQL<'b, SQLiteValue<'b>>,
    ) -> DrizzleBuilder<'a, S, DeleteBuilder<'b, S, DeleteWhereSet, T>, DeleteWhereSet> {
        let builder = self.builder.r#where(condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}
