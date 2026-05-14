// The typestate-advancing methods on `TransactionBuilder` thread the
// `SelectBuilder<.., State, T, M, R, G>` (and analogous Insert/Update/
// Delete) types through every level of the chain; spelling out the full
// generic in each return type is the surface this module deliberately
// exposes. Clippy's `type_complexity` lint is at odds with that shape.
#![allow(clippy::type_complexity)]

//! Shared typestate builder for PostgreSQL transaction wrappers.
//!
//! Mirrors `src/transaction/sqlite/typestate.rs` for the Postgres family.
//! `postgres_sync`, `tokio_postgres`, and `aws_data_api` previously each
//! defined their own `TransactionBuilder<'a, [..], Schema, Builder, State>`
//! struct and hand-wrote per-driver `delete.rs` / `insert.rs` /
//! `update.rs` / `select.rs` typestate files. The method bodies were pure
//! typestate plumbing — `let builder = self.builder.X(args); Self { ... }`
//! — and didn't touch anything driver-specific.
//!
//! This module unifies them with one generic [`TransactionBuilder`] keyed
//! on `Tx`. The `'conn` lifetime lives inside each driver's
//! `Transaction<'conn, Schema>` and is invisible at the typestate layer.
//! Drivers expose the generic via a per-driver type alias:
//!
//! ```ignore
//! // sync (postgres_sync) and async (tokio_postgres): both carry 'conn
//! pub type TransactionBuilder<'tx, 'conn, S, B, St> =
//!     typestate::TransactionBuilder<'tx, Transaction<'conn, S>, S, B, St>;
//!
//! // AWS Aurora Data API: HTTP-based, no 'conn
//! pub type TransactionBuilder<'tx, S, B, St> =
//!     typestate::TransactionBuilder<'tx, Transaction<S>, S, B, St>;
//! ```
//!
//! As a side benefit, `aws_data_api` — which previously had a
//! `TransactionBuilder` struct but no typestate-advancing methods on it —
//! picks up the full `.value` / `.values` / `.r#where` / `.set` /
//! `.on_conflict` / `.returning` / `.from` / `.join` / `.group_by` /
//! `.having` / `.order_by` / `.limit` / `.offset` / `.union[_all]` /
//! `.intersect[_all]` / `.except[_all]` / `.into_cte` surface for free.

use std::marker::PhantomData;

use drizzle_core::traits::SQLTable;
use drizzle_core::{
    ConflictTarget, IntoSelectTarget, NamedConstraint, ResolveRow, SQLModel, ScopePush, ToSQL,
};
use drizzle_postgres::builder::{
    CTEView, DeleteInitial, DeleteReturningSet, DeleteWhereSet, ExecutableState, InsertDoUpdateSet,
    InsertInitial, InsertOnConflictSet, InsertReturningSet, InsertValuesSet, OnConflictBuilder,
    SelectFromSet, SelectGroupSet, SelectInitial, SelectJoinSet, SelectLimitSet, SelectOffsetSet,
    SelectOrderSet, SelectSetOpSet, SelectWhereSet, UpdateInitial, UpdateReturningSet,
    UpdateSetClauseSet, UpdateWhereSet,
    delete::DeleteBuilder,
    insert::InsertBuilder,
    select::{AsCteState, IntoSelect, SelectBuilder},
    update::UpdateBuilder,
};
use drizzle_postgres::common::PostgresSchemaType;
use drizzle_postgres::helpers::JoinArg;
use drizzle_postgres::traits::PostgresTable;
use drizzle_postgres::values::PostgresValue;

/// Generic transaction-scoped query builder.
///
/// Holds a borrow of the driver-specific transaction (`Tx`) plus an
/// in-flight `drizzle_postgres::builder::*Builder` and its typestate
/// marker. The typestate-advancing methods are implemented below as
/// inherent impls keyed on the inner builder's typestate; the executor
/// methods (`.execute()`, `.all()`, `.rows()`, `.get()`) live per-driver
/// because they need to reach into `Tx` to actually run SQL.
#[derive(Debug)]
pub struct TransactionBuilder<'tx, Tx: ?Sized, Schema, Builder, State> {
    pub(crate) transaction: &'tx Tx,
    pub(crate) builder: Builder,
    pub(crate) _phantom: PhantomData<(Schema, State)>,
}

/// Generic mid-chain builder for typed `ON CONFLICT (...)` clauses.
#[derive(Debug)]
pub struct TransactionOnConflictBuilder<'tx, 'q, Tx: ?Sized, Schema, Table> {
    pub(crate) transaction: &'tx Tx,
    pub(crate) builder: OnConflictBuilder<'q, Schema, Table>,
}

// =============================================================================
// RETURNING return-type helpers
// =============================================================================

type ReturningMarker<Table, Columns> = drizzle_core::Scoped<
    <Columns as IntoSelectTarget>::Marker,
    drizzle_core::Cons<Table, drizzle_core::Nil>,
>;

type ReturningRow<Table, Columns> =
    <<Columns as IntoSelectTarget>::Marker as ResolveRow<Table>>::Row;

type DeleteReturningTxBuilder<'tx, 'q, Tx, S, T, Columns> = TransactionBuilder<
    'tx,
    Tx,
    S,
    DeleteBuilder<
        'q,
        S,
        DeleteReturningSet,
        T,
        ReturningMarker<T, Columns>,
        ReturningRow<T, Columns>,
    >,
    DeleteReturningSet,
>;

type UpdateReturningTxBuilder<'tx, 'q, Tx, S, T, Columns> = TransactionBuilder<
    'tx,
    Tx,
    S,
    UpdateBuilder<
        'q,
        S,
        UpdateReturningSet,
        T,
        ReturningMarker<T, Columns>,
        ReturningRow<T, Columns>,
    >,
    UpdateReturningSet,
>;

type InsertReturningTxBuilder<'tx, 'q, Tx, S, T, Columns> = TransactionBuilder<
    'tx,
    Tx,
    S,
    InsertBuilder<
        'q,
        S,
        InsertReturningSet,
        T,
        ReturningMarker<T, Columns>,
        ReturningRow<T, Columns>,
    >,
    InsertReturningSet,
>;

// =============================================================================
// DELETE typestate
// =============================================================================

impl<'tx, 'q, Tx: ?Sized, S, T>
    TransactionBuilder<'tx, Tx, S, DeleteBuilder<'q, S, DeleteInitial, T>, DeleteInitial>
where
    T: PostgresTable<'q>,
{
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> TransactionBuilder<'tx, Tx, S, DeleteBuilder<'q, S, DeleteWhereSet, T>, DeleteWhereSet>
    where
        E: drizzle_core::expr::Expr<'q, PostgresValue<'q>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        let builder = self.builder.r#where(condition);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> DeleteReturningTxBuilder<'tx, 'q, Tx, S, T, Columns>
    where
        Columns: ToSQL<'q, PostgresValue<'q>> + IntoSelectTarget,
        Columns::Marker: ResolveRow<T>,
    {
        let builder = self.builder.returning(columns);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

impl<'tx, 'q, Tx: ?Sized, S, T>
    TransactionBuilder<'tx, Tx, S, DeleteBuilder<'q, S, DeleteWhereSet, T>, DeleteWhereSet>
{
    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> DeleteReturningTxBuilder<'tx, 'q, Tx, S, T, Columns>
    where
        Columns: ToSQL<'q, PostgresValue<'q>> + IntoSelectTarget,
        Columns::Marker: ResolveRow<T>,
    {
        let builder = self.builder.returning(columns);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

// =============================================================================
// UPDATE typestate
// =============================================================================

impl<'tx, 'q, Tx: ?Sized, S, T>
    TransactionBuilder<'tx, Tx, S, UpdateBuilder<'q, S, UpdateInitial, T>, UpdateInitial>
where
    T: PostgresTable<'q>,
{
    #[inline]
    pub fn set(
        self,
        values: T::Update,
    ) -> TransactionBuilder<
        'tx,
        Tx,
        S,
        UpdateBuilder<'q, S, UpdateSetClauseSet, T>,
        UpdateSetClauseSet,
    > {
        let builder = self.builder.set(values);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

impl<'tx, 'q, Tx: ?Sized, S, T>
    TransactionBuilder<'tx, Tx, S, UpdateBuilder<'q, S, UpdateSetClauseSet, T>, UpdateSetClauseSet>
{
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> TransactionBuilder<'tx, Tx, S, UpdateBuilder<'q, S, UpdateWhereSet, T>, UpdateWhereSet>
    where
        E: drizzle_core::expr::Expr<'q, PostgresValue<'q>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        let builder = self.builder.r#where(condition);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> UpdateReturningTxBuilder<'tx, 'q, Tx, S, T, Columns>
    where
        Columns: ToSQL<'q, PostgresValue<'q>> + IntoSelectTarget,
        Columns::Marker: ResolveRow<T>,
    {
        let builder = self.builder.returning(columns);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

impl<'tx, 'q, Tx: ?Sized, S, T>
    TransactionBuilder<'tx, Tx, S, UpdateBuilder<'q, S, UpdateWhereSet, T>, UpdateWhereSet>
{
    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> UpdateReturningTxBuilder<'tx, 'q, Tx, S, T, Columns>
    where
        Columns: ToSQL<'q, PostgresValue<'q>> + IntoSelectTarget,
        Columns::Marker: ResolveRow<T>,
    {
        let builder = self.builder.returning(columns);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

// =============================================================================
// INSERT typestate
// =============================================================================

impl<'tx, 'q, Tx: ?Sized, S, T> TransactionOnConflictBuilder<'tx, 'q, Tx, S, T> {
    /// Adds a WHERE clause to the conflict target for partial index matching.
    pub fn r#where<E>(mut self, condition: E) -> Self
    where
        E: drizzle_core::expr::Expr<'q, PostgresValue<'q>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        self.builder = self.builder.r#where(condition);
        self
    }

    /// `ON CONFLICT (cols) DO NOTHING`
    pub fn do_nothing(
        self,
    ) -> TransactionBuilder<
        'tx,
        Tx,
        S,
        InsertBuilder<'q, S, InsertOnConflictSet, T>,
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
        set: impl ToSQL<'q, PostgresValue<'q>>,
    ) -> TransactionBuilder<'tx, Tx, S, InsertBuilder<'q, S, InsertDoUpdateSet, T>, InsertDoUpdateSet>
    {
        TransactionBuilder {
            transaction: self.transaction,
            builder: self.builder.do_update(set),
            _phantom: PhantomData,
        }
    }
}

impl<'tx, 'q, Tx: ?Sized, S, T>
    TransactionBuilder<'tx, Tx, S, InsertBuilder<'q, S, InsertInitial, T>, InsertInitial>
{
    #[inline]
    pub fn value<V>(
        self,
        value: T::Insert<V>,
    ) -> TransactionBuilder<'tx, Tx, S, InsertBuilder<'q, S, InsertValuesSet, T>, InsertValuesSet>
    where
        T: PostgresTable<'q>,
        T::Insert<V>: SQLModel<'q, PostgresValue<'q>>,
    {
        self.values([value])
    }

    #[inline]
    pub fn values<V>(
        self,
        values: impl IntoIterator<Item = T::Insert<V>>,
    ) -> TransactionBuilder<'tx, Tx, S, InsertBuilder<'q, S, InsertValuesSet, T>, InsertValuesSet>
    where
        T: PostgresTable<'q>,
        T::Insert<V>: SQLModel<'q, PostgresValue<'q>>,
    {
        let builder = self.builder.values(values);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

impl<'tx, 'q, Tx: ?Sized, S, T>
    TransactionBuilder<'tx, Tx, S, InsertBuilder<'q, S, InsertValuesSet, T>, InsertValuesSet>
where
    T: PostgresTable<'q>,
{
    /// Begins a typed ON CONFLICT clause targeting specific columns.
    pub fn on_conflict<C: ConflictTarget<T>>(
        self,
        target: C,
    ) -> TransactionOnConflictBuilder<'tx, 'q, Tx, S, T> {
        TransactionOnConflictBuilder {
            transaction: self.transaction,
            builder: self.builder.on_conflict(target),
        }
    }

    /// Begins a typed ON CONFLICT ON CONSTRAINT clause (PostgreSQL-only).
    pub fn on_conflict_on_constraint<C: NamedConstraint<T>>(
        self,
        target: C,
    ) -> TransactionOnConflictBuilder<'tx, 'q, Tx, S, T> {
        TransactionOnConflictBuilder {
            transaction: self.transaction,
            builder: self.builder.on_conflict_on_constraint(target),
        }
    }

    /// Shorthand for `ON CONFLICT DO NOTHING` without specifying a target.
    pub fn on_conflict_do_nothing(
        self,
    ) -> TransactionBuilder<
        'tx,
        Tx,
        S,
        InsertBuilder<'q, S, InsertOnConflictSet, T>,
        InsertOnConflictSet,
    > {
        TransactionBuilder {
            transaction: self.transaction,
            builder: self.builder.on_conflict_do_nothing(),
            _phantom: PhantomData,
        }
    }

    /// Adds RETURNING clause
    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> InsertReturningTxBuilder<'tx, 'q, Tx, S, T, Columns>
    where
        Columns: ToSQL<'q, PostgresValue<'q>> + IntoSelectTarget,
        Columns::Marker: ResolveRow<T>,
    {
        let builder = self.builder.returning(columns);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

impl<'tx, 'q, Tx: ?Sized, S, T>
    TransactionBuilder<
        'tx,
        Tx,
        S,
        InsertBuilder<'q, S, InsertOnConflictSet, T>,
        InsertOnConflictSet,
    >
{
    /// Adds RETURNING clause after ON CONFLICT
    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> InsertReturningTxBuilder<'tx, 'q, Tx, S, T, Columns>
    where
        Columns: ToSQL<'q, PostgresValue<'q>> + IntoSelectTarget,
        Columns::Marker: ResolveRow<T>,
    {
        let builder = self.builder.returning(columns);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

impl<'tx, 'q, Tx: ?Sized, S, T>
    TransactionBuilder<'tx, Tx, S, InsertBuilder<'q, S, InsertDoUpdateSet, T>, InsertDoUpdateSet>
{
    /// Adds WHERE clause after DO UPDATE SET
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> TransactionBuilder<
        'tx,
        Tx,
        S,
        InsertBuilder<'q, S, InsertOnConflictSet, T>,
        InsertOnConflictSet,
    >
    where
        E: drizzle_core::expr::Expr<'q, PostgresValue<'q>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        TransactionBuilder {
            transaction: self.transaction,
            builder: self.builder.r#where(condition),
            _phantom: PhantomData,
        }
    }

    /// Adds RETURNING clause after DO UPDATE SET
    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> InsertReturningTxBuilder<'tx, 'q, Tx, S, T, Columns>
    where
        Columns: ToSQL<'q, PostgresValue<'q>> + IntoSelectTarget,
        Columns::Marker: ResolveRow<T>,
    {
        let builder = self.builder.returning(columns);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

// =============================================================================
// SELECT typestate
// =============================================================================

impl<'tx, 'q, Tx: ?Sized, Schema, M>
    TransactionBuilder<
        'tx,
        Tx,
        Schema,
        SelectBuilder<'q, Schema, SelectInitial, (), M>,
        SelectInitial,
    >
{
    #[inline]
    pub fn from<T>(
        self,
        table: T,
    ) -> TransactionBuilder<
        'tx,
        Tx,
        Schema,
        SelectBuilder<
            'q,
            Schema,
            SelectFromSet,
            T,
            drizzle_core::Scoped<M, drizzle_core::Cons<T, drizzle_core::Nil>>,
            <M as ResolveRow<T>>::Row,
        >,
        SelectFromSet,
    >
    where
        T: ToSQL<'q, PostgresValue<'q>>,
        M: ResolveRow<T>,
    {
        let builder = self.builder.from(table);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

/// Emit a method on `TransactionBuilder<.. SelectBuilder<state, ..>, state>`
/// that forwards to the underlying builder, preserving the `Tx` parameter.
macro_rules! impl_tx_select_methods {
    ($($state:ty => [$($method:ident),* $(,)?]),+ $(,)?) => {
        $(
            impl<'tx, 'q, Tx: ?Sized, Schema, T, M, R, G>
                TransactionBuilder<'tx, Tx, Schema, SelectBuilder<'q, Schema, $state, T, M, R, G>, $state>
            { $( impl_tx_select_methods!(@method $method); )* }
        )+
    };
    (@method r#where) => {
        #[inline]
        pub fn r#where<E>(
            self,
            condition: E,
        ) -> TransactionBuilder<'tx, Tx, Schema, SelectBuilder<'q, Schema, SelectWhereSet, T, M, R, G>, SelectWhereSet>
        where
            E: drizzle_core::expr::Expr<'q, PostgresValue<'q>>,
            E::SQLType: drizzle_core::types::BooleanLike,
        {
            let builder = self.builder.r#where(condition);
            TransactionBuilder { transaction: self.transaction, builder, _phantom: PhantomData }
        }
    };
    (@method group_by) => {
        pub fn group_by<Gr>(
            self,
            columns: Gr,
        ) -> TransactionBuilder<'tx, Tx, Schema, SelectBuilder<'q, Schema, SelectGroupSet, T, M, R, Gr::Columns>, SelectGroupSet>
        where
            Gr: drizzle_core::IntoGroupBy<'q, PostgresValue<'q>>,
        {
            let builder = self.builder.group_by(columns);
            TransactionBuilder { transaction: self.transaction, builder, _phantom: PhantomData }
        }
    };
    (@method having) => {
        pub fn having<E>(
            self,
            condition: E,
        ) -> TransactionBuilder<'tx, Tx, Schema, SelectBuilder<'q, Schema, SelectGroupSet, T, M, R, G>, SelectGroupSet>
        where
            E: drizzle_core::expr::Expr<'q, PostgresValue<'q>>,
            E::SQLType: drizzle_core::types::BooleanLike,
        {
            let builder = self.builder.having(condition);
            TransactionBuilder { transaction: self.transaction, builder, _phantom: PhantomData }
        }
    };
    (@method order_by) => {
        pub fn order_by<TOrderBy>(
            self,
            expressions: TOrderBy,
        ) -> TransactionBuilder<'tx, Tx, Schema, SelectBuilder<'q, Schema, SelectOrderSet, T, M, R, G>, SelectOrderSet>
        where
            TOrderBy: ToSQL<'q, PostgresValue<'q>>,
        {
            let builder = self.builder.order_by(expressions);
            TransactionBuilder { transaction: self.transaction, builder, _phantom: PhantomData }
        }
    };
    (@method limit) => {
        #[inline]
        pub fn limit(
            self,
            limit: usize,
        ) -> TransactionBuilder<'tx, Tx, Schema, SelectBuilder<'q, Schema, SelectLimitSet, T, M, R, G>, SelectLimitSet>
        {
            let builder = self.builder.limit(limit);
            TransactionBuilder { transaction: self.transaction, builder, _phantom: PhantomData }
        }
    };
    (@method offset) => {
        pub fn offset(
            self,
            offset: usize,
        ) -> TransactionBuilder<'tx, Tx, Schema, SelectBuilder<'q, Schema, SelectOffsetSet, T, M, R, G>, SelectOffsetSet>
        {
            let builder = self.builder.offset(offset);
            TransactionBuilder { transaction: self.transaction, builder, _phantom: PhantomData }
        }
    };
    (@method join) => {
        #[inline]
        pub fn join<J: JoinArg<'q, T>>(
            self,
            arg: J,
        ) -> TransactionBuilder<
            'tx,
            Tx,
            Schema,
            SelectBuilder<'q, Schema, SelectJoinSet, J::JoinedTable, <M as ScopePush<J::JoinedTable>>::Out, <M as drizzle_core::AfterJoin<R, J::JoinedTable>>::NewRow, G>,
            SelectJoinSet,
        >
        where
            M: drizzle_core::AfterJoin<R, J::JoinedTable> + ScopePush<J::JoinedTable>,
        {
            let builder = self.builder.join(arg);
            TransactionBuilder { transaction: self.transaction, builder, _phantom: PhantomData }
        }
    };
}

impl_tx_select_methods! {
    SelectFromSet  => [r#where, group_by, order_by, limit, offset, join],
    SelectJoinSet  => [r#where, group_by, order_by, join],
    SelectWhereSet => [group_by, order_by, limit],
    SelectGroupSet => [having, order_by, limit],
    SelectOrderSet => [limit],
    SelectLimitSet => [offset],
    SelectSetOpSet => [order_by, limit, offset],
}

// -----------------------------------------------------------------------------
// Set operations on TransactionBuilder
// -----------------------------------------------------------------------------

macro_rules! impl_tx_select_set_op {
    ($($op:ident),* $(,)?) => {
        $(
            #[doc = concat!(" `", stringify!($op), "` — compose with another SELECT producing the same row marker.")]
            pub fn $op(
                self,
                other: impl IntoSelect<'q, Schema, M, R>,
            ) -> TransactionBuilder<
                'tx,
                Tx,
                Schema,
                SelectBuilder<'q, Schema, SelectSetOpSet, T, M, R, G>,
                SelectSetOpSet,
            > {
                TransactionBuilder {
                    transaction: self.transaction,
                    builder: self.builder.$op(other),
                    _phantom: PhantomData,
                }
            }
        )*
    };
}

impl<'tx, 'q, Tx: ?Sized, Schema, State, T, M, R, G>
    TransactionBuilder<'tx, Tx, Schema, SelectBuilder<'q, Schema, State, T, M, R, G>, State>
where
    State: ExecutableState,
{
    impl_tx_select_set_op!(
        union,
        union_all,
        intersect,
        intersect_all,
        except,
        except_all
    );
}

// -----------------------------------------------------------------------------
// into_cte
// -----------------------------------------------------------------------------

impl<'tx, 'q, Tx: ?Sized, Schema, State, T, M, R, G>
    TransactionBuilder<'tx, Tx, Schema, SelectBuilder<'q, Schema, State, T, M, R, G>, State>
where
    State: AsCteState,
    T: SQLTable<'q, PostgresSchemaType, PostgresValue<'q>>,
{
    #[inline]
    pub fn into_cte<Tag: drizzle_core::Tag + 'static>(
        self,
    ) -> CTEView<
        'q,
        <T as SQLTable<'q, PostgresSchemaType, PostgresValue<'q>>>::Aliased<Tag>,
        SelectBuilder<'q, Schema, State, T, M, R, G>,
    > {
        self.builder.into_cte::<Tag>()
    }
}
