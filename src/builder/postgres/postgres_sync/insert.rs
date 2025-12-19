use std::marker::PhantomData;

use drizzle_core::traits::{SQLModel, ToSQL};
use drizzle_postgres::{
    PostgresValue,
    builder::{
        Conflict, InsertInitial, InsertOnConflictSet, InsertReturningSet, InsertValuesSet,
        insert::InsertBuilder,
    },
    traits::PostgresTable,
};

use crate::builder::postgres::postgres_sync::DrizzleBuilder;

impl<'a, 'b, Schema, Table>
    DrizzleBuilder<'a, Schema, InsertBuilder<'b, Schema, InsertInitial, Table>, InsertInitial>
{
    #[inline]
    pub fn values<T>(
        self,
        values: impl IntoIterator<Item = Table::Insert<T>>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        InsertBuilder<'b, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
    where
        Table: PostgresTable<'b>,
        Table::Insert<T>: SQLModel<'b, PostgresValue<'b>>,
    {
        let builder = self.builder.values(values);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// INSERT ValuesSet State Implementation
//------------------------------------------------------------------------------

impl<'a, 'b, Schema, Table>
    DrizzleBuilder<'a, Schema, InsertBuilder<'b, Schema, InsertValuesSet, Table>, InsertValuesSet>
where
    Table: PostgresTable<'b>,
{
    /// Adds conflict resolution clause
    pub fn on_conflict(
        self,
        conflict: Conflict<'b>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        InsertBuilder<'b, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    > {
        let builder = self.builder.on_conflict(conflict);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    /// Adds RETURNING clause
    pub fn returning(
        self,
        columns: impl ToSQL<'b, PostgresValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        InsertBuilder<'b, Schema, InsertReturningSet, Table>,
        InsertReturningSet,
    > {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// INSERT OnConflict State Implementation
//------------------------------------------------------------------------------

impl<'a, 'b, Schema, Table>
    DrizzleBuilder<
        'a,
        Schema,
        InsertBuilder<'b, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    >
{
    /// Adds RETURNING clause after ON CONFLICT
    pub fn returning(
        self,
        columns: impl ToSQL<'b, PostgresValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        InsertBuilder<'b, Schema, InsertReturningSet, Table>,
        InsertReturningSet,
    > {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}
