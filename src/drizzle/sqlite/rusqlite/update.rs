use std::marker::PhantomData;

use drizzle_sqlite::{
    SQLiteValue,
    builder::{UpdateInitial, UpdateSetClauseSet, UpdateWhereSet, update::UpdateBuilder},
    traits::SQLiteTable,
};

use crate::drizzle::sqlite::rusqlite::DrizzleBuilder;

impl<'a, Schema, Table>
    DrizzleBuilder<'a, Schema, UpdateBuilder<'a, Schema, UpdateInitial, Table>, UpdateInitial>
where
    Table: SQLiteTable<'a>,
{
    #[inline]
    pub fn set(
        self,
        values: Table::Update,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        UpdateBuilder<'a, Schema, UpdateSetClauseSet, Table>,
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

impl<'a, Schema, Table>
    DrizzleBuilder<
        'a,
        Schema,
        UpdateBuilder<'a, Schema, UpdateSetClauseSet, Table>,
        UpdateSetClauseSet,
    >
{
    pub fn r#where(
        self,
        condition: drizzle_core::SQL<'a, SQLiteValue<'a>>,
    ) -> DrizzleBuilder<'a, Schema, UpdateBuilder<'a, Schema, UpdateWhereSet, Table>, UpdateWhereSet>
    {
        let builder = self.builder.r#where(condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}
