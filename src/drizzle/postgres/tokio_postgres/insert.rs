use std::marker::PhantomData;

use drizzle_core::{SQLModel, ToSQL};
use drizzle_postgres::{
    PostgresValue,
    builder::{
        Conflict, InsertInitial, InsertOnConflictSet, InsertReturningSet, InsertValuesSet,
        insert::InsertBuilder,
    },
    traits::PostgresTable,
};

use crate::drizzle::postgres::tokio_postgres::DrizzleBuilder;

impl<'a, Schema, Table>
    DrizzleBuilder<'a, Schema, InsertBuilder<'a, Schema, InsertInitial, Table>, InsertInitial>
{
    #[inline]
    pub fn values<T>(
        self,
        values: impl IntoIterator<Item = Table::Insert<T>>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        InsertBuilder<'a, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
    where
        Table: PostgresTable<'a>,
        Table::Insert<T>: SQLModel<'a, PostgresValue<'a>>,
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

impl<'a, Schema, Table>
    DrizzleBuilder<'a, Schema, InsertBuilder<'a, Schema, InsertValuesSet, Table>, InsertValuesSet>
where
    Table: PostgresTable<'a>,
{
    /// Adds conflict resolution clause
    pub fn on_conflict(
        self,
        conflict: Conflict<'a>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        InsertBuilder<'a, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    >
    {
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
        columns: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        InsertBuilder<'a, Schema, InsertReturningSet, Table>,
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

impl<'a, Schema, Table>
    DrizzleBuilder<
        'a,
        Schema,
        InsertBuilder<'a, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    >
{
    /// Adds RETURNING clause after ON CONFLICT
    pub fn returning(
        self,
        columns: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        InsertBuilder<'a, Schema, InsertReturningSet, Table>,
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
