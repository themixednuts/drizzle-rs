use crate::transaction::sqlite::rusqlite::TransactionBuilder;
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

impl<'a, 'conn, Schema, Table>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertInitial, Table>,
        InsertInitial,
    >
{
    #[inline]
    pub fn values<T>(
        self,
        values: impl IntoIterator<Item = Table::Insert<T>>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
    where
        Table: SQLiteTable<'a>,
        Table::Insert<T>: SQLModel<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.values(values);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

impl<'a, 'conn, Schema, Table>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
where
    Table: SQLiteTable<'a>,
{
    /// Adds conflict resolution clause
    pub fn on_conflict<TI>(
        self,
        conflict: Conflict<'a, TI>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    >
    where
        TI: IntoIterator,
        TI::Item: ToSQL<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.on_conflict(conflict);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Adds RETURNING clause
    pub fn returning(
        self,
        columns: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertReturningSet, Table>,
        InsertReturningSet,
    > {
        let builder = self.builder.returning(columns);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

impl<'a, 'conn, Schema, Table>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    >
{
    /// Adds RETURNING clause after ON CONFLICT
    pub fn returning(
        self,
        columns: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertReturningSet, Table>,
        InsertReturningSet,
    > {
        let builder = self.builder.returning(columns);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}
