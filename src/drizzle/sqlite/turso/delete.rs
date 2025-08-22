use std::marker::PhantomData;

use drizzle_core::{IsInSchema, SQLTable};
use drizzle_sqlite::{
    SQLiteValue,
    builder::{DeleteInitial, DeleteWhereSet, delete::DeleteBuilder},
};

use crate::drizzle::sqlite::turso::DrizzleBuilder;

impl<'a, S, T> DrizzleBuilder<'a, S, DeleteBuilder<'a, S, DeleteInitial, T>, DeleteInitial>
where
    T: IsInSchema<S> + SQLTable<'a, SQLiteValue<'a>>,
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
