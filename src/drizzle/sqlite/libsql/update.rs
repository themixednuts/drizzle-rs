use crate::drizzle::sqlite::libsql::DrizzleBuilder;
use drizzle_sqlite::{
    SQLiteValue,
    builder::{UpdateInitial, UpdateSetClauseSet, UpdateWhereSet, update::UpdateBuilder},
    traits::SQLiteTable,
};
use std::marker::PhantomData;

impl<'a, 'b, Schema, Table>
    DrizzleBuilder<'a, Schema, UpdateBuilder<'b, Schema, UpdateInitial, Table>, UpdateInitial>
where
    Table: SQLiteTable<'b>,
{
    #[inline]
    pub fn set(
        self,
        values: Table::Update,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        UpdateBuilder<'b, Schema, UpdateSetClauseSet, Table>,
        UpdateSetClauseSet,
    > {
        let builder = self.builder.set(values);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Schema, Table>
    DrizzleBuilder<
        'a,
        Schema,
        UpdateBuilder<'b, Schema, UpdateSetClauseSet, Table>,
        UpdateSetClauseSet,
    >
{
    pub fn r#where(
        self,
        condition: drizzle_core::SQL<'b, SQLiteValue<'b>>,
    ) -> DrizzleBuilder<'a, Schema, UpdateBuilder<'b, Schema, UpdateWhereSet, Table>, UpdateWhereSet>
    {
        let builder = self.builder.r#where(condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}
