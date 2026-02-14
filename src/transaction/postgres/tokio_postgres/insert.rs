use crate::transaction::postgres::tokio_postgres::TransactionBuilder;
use drizzle_core::{ConflictTarget, NamedConstraint, SQLModel, ToSQL};
use drizzle_postgres::builder::{
    InsertDoUpdateSet, InsertInitial, InsertOnConflictSet, InsertReturningSet, InsertValuesSet,
    OnConflictBuilder, insert::InsertBuilder,
};
use drizzle_postgres::traits::PostgresTable;
use drizzle_postgres::values::PostgresValue;
use std::marker::PhantomData;

/// Intermediate builder for typed ON CONFLICT within a tokio-postgres transaction.
pub struct TransactionOnConflictBuilder<'a, 'conn, Schema, Table> {
    transaction: &'a super::Transaction<'conn, Schema>,
    builder: OnConflictBuilder<'a, Schema, Table>,
}

impl<'a, 'conn, Schema, Table> TransactionOnConflictBuilder<'a, 'conn, Schema, Table> {
    /// Adds a WHERE clause to the conflict target for partial index matching.
    pub fn r#where(mut self, condition: impl ToSQL<'a, PostgresValue<'a>>) -> Self {
        self.builder = self.builder.r#where(condition);
        self
    }

    /// `ON CONFLICT (cols) DO NOTHING`
    pub fn do_nothing(
        self,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    > {
        TransactionBuilder {
            transaction: self.transaction,
            builder: self.builder.do_nothing(),
            _phantom: PhantomData,
        }
    }

    /// `ON CONFLICT (cols) DO UPDATE SET ...`
    pub fn do_update(
        self,
        set: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertDoUpdateSet, Table>,
        InsertDoUpdateSet,
    > {
        TransactionBuilder {
            transaction: self.transaction,
            builder: self.builder.do_update(set),
            _phantom: PhantomData,
        }
    }
}

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
        Table: PostgresTable<'a>,
        Table::Insert<T>: SQLModel<'a, PostgresValue<'a>>,
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
    Table: PostgresTable<'a>,
{
    /// Begins a typed ON CONFLICT clause targeting specific columns.
    pub fn on_conflict<C: ConflictTarget<Table>>(
        self,
        target: C,
    ) -> TransactionOnConflictBuilder<'a, 'conn, Schema, Table> {
        TransactionOnConflictBuilder {
            transaction: self.transaction,
            builder: self.builder.on_conflict(target),
        }
    }

    /// Begins a typed ON CONFLICT ON CONSTRAINT clause (PostgreSQL-only).
    pub fn on_conflict_on_constraint<C: NamedConstraint<Table>>(
        self,
        target: C,
    ) -> TransactionOnConflictBuilder<'a, 'conn, Schema, Table> {
        TransactionOnConflictBuilder {
            transaction: self.transaction,
            builder: self.builder.on_conflict_on_constraint(target),
        }
    }

    /// Shorthand for `ON CONFLICT DO NOTHING` without specifying a target.
    pub fn on_conflict_do_nothing(
        self,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    > {
        TransactionBuilder {
            transaction: self.transaction,
            builder: self.builder.on_conflict_do_nothing(),
            _phantom: PhantomData,
        }
    }

    /// Adds RETURNING clause
    pub fn returning(
        self,
        columns: impl ToSQL<'a, PostgresValue<'a>>,
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
        columns: impl ToSQL<'a, PostgresValue<'a>>,
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
        InsertBuilder<'a, Schema, InsertDoUpdateSet, Table>,
        InsertDoUpdateSet,
    >
{
    /// Adds WHERE clause after DO UPDATE SET
    pub fn r#where(
        self,
        condition: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    > {
        TransactionBuilder {
            transaction: self.transaction,
            builder: self.builder.r#where(condition),
            _phantom: PhantomData,
        }
    }

    /// Adds RETURNING clause after DO UPDATE SET
    pub fn returning(
        self,
        columns: impl ToSQL<'a, PostgresValue<'a>>,
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
