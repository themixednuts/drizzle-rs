use crate::drizzle::sqlite::libsql::DrizzleBuilder;
use drizzle_core::{SQLModel, ToSQL};
use drizzle_sqlite::{
    SQLiteValue,
    builder::{
        Conflict, InsertInitial, InsertOnConflictSet, InsertReturningSet, InsertValuesSet,
        insert::InsertBuilder,
    },
    traits::SQLiteTable,
};
use std::marker::PhantomData;

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
        Table: SQLiteTable<'b>,
        Table::Insert<T>: SQLModel<'b, SQLiteValue<'b>>,
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
    Table: SQLiteTable<'b>,
{
    /// Adds conflict resolution clause
    pub fn on_conflict<TI>(
        self,
        conflict: Conflict<'b, TI>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        InsertBuilder<'b, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    >
    where
        TI: IntoIterator,
        TI::Item: ToSQL<'b, SQLiteValue<'b>>,
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
        columns: impl ToSQL<'b, SQLiteValue<'b>>,
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
        columns: impl ToSQL<'b, SQLiteValue<'b>>,
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
