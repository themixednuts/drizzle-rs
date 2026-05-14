// The typestate-advancing methods on `TransactionBuilder` thread the
// `SelectBuilder<.., State, T, M, R, G>` type through every level of the
// chain; spelling out the full generic in each return type is part of
// the surface this module deliberately exposes (callers see one long
// type, not an avalanche of intermediate aliases). Clippy's
// `type_complexity` lint is at odds with that shape, so opt out at the
// module level.
#![allow(clippy::type_complexity)]

//! Shared typestate builder for SQLite transaction wrappers.
//!
//! Every SQLite driver (`rusqlite`, `libsql`, `turso`, `durable`) wraps its
//! `Transaction` in a per-driver `TransactionBuilder<...>` typestate struct,
//! then mirrors the same set of typestate-advancing builder methods that
//! `drizzle_sqlite::builder` exposes on the outer `QueryBuilder`. The method
//! bodies are pure typestate plumbing — `let builder = self.builder.X(args);
//! Self { ... }` — and don't touch anything driver-specific.
//!
//! Hand-written per driver, that's ~1,000 lines of near-clones across four
//! drivers. The only structural variation is whether the surrounding
//! `Transaction` type carries a `'conn` lifetime (sync drivers like rusqlite
//! do; async drivers like libsql don't).
//!
//! This module unifies them by introducing one generic
//! [`TransactionBuilder`] keyed on a `Tx` parameter. The `'conn` lifetime
//! lives *inside* each driver's `Transaction<'conn, Schema>` type and is
//! invisible at the typestate layer. Drivers expose the generic via a
//! per-driver type alias:
//!
//! ```ignore
//! // sync (rusqlite, turso): 'conn is part of the public alias signature
//! pub type TransactionBuilder<'tx, 'conn, S, B, St> =
//!     typestate::TransactionBuilder<'tx, Transaction<'conn, S>, S, B, St>;
//!
//! // async (libsql, durable): no 'conn
//! pub type TransactionBuilder<'tx, S, B, St> =
//!     typestate::TransactionBuilder<'tx, Transaction<S>, S, B, St>;
//! ```
//!
//! Driver-specific executor methods (`.execute()`, `.all()`, `.rows()`,
//! `.get()`) live in each driver's `mod.rs` as inherent impls keyed on
//! the concrete `Tx`. The typestate plumbing here doesn't see them.

use std::marker::PhantomData;

use drizzle_core::{ConflictTarget, IntoSelectTarget, ResolveRow, SQLModel, ScopePush, ToSQL};
use drizzle_sqlite::builder::{
    CTEView, DeleteInitial, DeleteWhereSet, ExecutableState, InsertDoUpdateSet, InsertInitial,
    InsertOnConflictSet, InsertReturningSet, InsertValuesSet, OnConflictBuilder, SelectFromSet,
    SelectGroupSet, SelectInitial, SelectJoinSet, SelectLimitSet, SelectOffsetSet, SelectOrderSet,
    SelectSetOpSet, SelectWhereSet, UpdateInitial, UpdateSetClauseSet, UpdateWhereSet,
    delete::DeleteBuilder,
    insert::InsertBuilder,
    select::{AsCteState, IntoSelect, SelectBuilder},
    update::UpdateBuilder,
};
use drizzle_sqlite::helpers::JoinArg;
use drizzle_sqlite::traits::SQLiteTable;
use drizzle_sqlite::values::SQLiteValue;

/// Generic transaction-scoped query builder.
///
/// Holds a borrow of the driver-specific transaction (`Tx`) plus an
/// in-flight `drizzle_sqlite::builder::*Builder` and its typestate marker.
/// The typestate-advancing methods (`.values()`, `.r#where()`, etc.) are
/// implemented below as inherent impls keyed on the inner builder's
/// typestate; the executor methods live per-driver because they need to
/// reach into `Tx` to actually run SQL.
#[derive(Debug)]
pub struct TransactionBuilder<'tx, Tx: ?Sized, Schema, Builder, State> {
    pub(crate) transaction: &'tx Tx,
    pub(crate) builder: Builder,
    pub(crate) _phantom: PhantomData<(Schema, State)>,
}

/// Generic mid-chain builder for typed `ON CONFLICT (...)` clauses.
///
/// Mirrors [`TransactionBuilder`]'s `Tx` parameter so it can be returned
/// without losing the driver association.
#[derive(Debug)]
pub struct TransactionOnConflictBuilder<'tx, 'a, Tx: ?Sized, Schema, Table> {
    pub(crate) transaction: &'tx Tx,
    pub(crate) builder: OnConflictBuilder<'a, Schema, Table>,
}

// =============================================================================
// Type aliases for RETURNING in INSERT
// =============================================================================

type ReturningMarker<Table, Columns> = drizzle_core::Scoped<
    <Columns as IntoSelectTarget>::Marker,
    drizzle_core::Cons<Table, drizzle_core::Nil>,
>;

type ReturningRow<Table, Columns> =
    <<Columns as IntoSelectTarget>::Marker as ResolveRow<Table>>::Row;

type InsertReturningTxBuilder<'tx, 'a, Tx, Schema, Table, Columns> = TransactionBuilder<
    'tx,
    Tx,
    Schema,
    InsertBuilder<
        'a,
        Schema,
        InsertReturningSet,
        Table,
        ReturningMarker<Table, Columns>,
        ReturningRow<Table, Columns>,
    >,
    InsertReturningSet,
>;

// =============================================================================
// DELETE typestate
// =============================================================================

impl<'tx, 'a, Tx: ?Sized, S, T>
    TransactionBuilder<'tx, Tx, S, DeleteBuilder<'a, S, DeleteInitial, T>, DeleteInitial>
where
    T: SQLiteTable<'a>,
{
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> TransactionBuilder<'tx, Tx, S, DeleteBuilder<'a, S, DeleteWhereSet, T>, DeleteWhereSet>
    where
        E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        let builder = self.builder.r#where(condition);
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

impl<'tx, 'a, Tx: ?Sized, S, T>
    TransactionBuilder<'tx, Tx, S, UpdateBuilder<'a, S, UpdateInitial, T>, UpdateInitial>
where
    T: SQLiteTable<'a>,
{
    #[inline]
    pub fn set(
        self,
        values: T::Update,
    ) -> TransactionBuilder<
        'tx,
        Tx,
        S,
        UpdateBuilder<'a, S, UpdateSetClauseSet, T>,
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

impl<'tx, 'a, Tx: ?Sized, S, T>
    TransactionBuilder<'tx, Tx, S, UpdateBuilder<'a, S, UpdateSetClauseSet, T>, UpdateSetClauseSet>
{
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> TransactionBuilder<'tx, Tx, S, UpdateBuilder<'a, S, UpdateWhereSet, T>, UpdateWhereSet>
    where
        E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        let builder = self.builder.r#where(condition);
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

impl<'tx, 'a, Tx: ?Sized, S, T> TransactionOnConflictBuilder<'tx, 'a, Tx, S, T> {
    /// Adds a WHERE clause to the conflict target for partial index matching.
    pub fn r#where<E>(mut self, condition: E) -> Self
    where
        E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
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
        InsertBuilder<'a, S, InsertOnConflictSet, T>,
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
        set: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> TransactionBuilder<'tx, Tx, S, InsertBuilder<'a, S, InsertDoUpdateSet, T>, InsertDoUpdateSet>
    {
        TransactionBuilder {
            transaction: self.transaction,
            builder: self.builder.do_update(set),
            _phantom: PhantomData,
        }
    }
}

impl<'tx, 'a, Tx: ?Sized, S, T>
    TransactionBuilder<'tx, Tx, S, InsertBuilder<'a, S, InsertInitial, T>, InsertInitial>
{
    #[inline]
    pub fn value<V>(
        self,
        value: T::Insert<V>,
    ) -> TransactionBuilder<'tx, Tx, S, InsertBuilder<'a, S, InsertValuesSet, T>, InsertValuesSet>
    where
        T: SQLiteTable<'a>,
        T::Insert<V>: SQLModel<'a, SQLiteValue<'a>>,
    {
        self.values([value])
    }

    #[inline]
    pub fn values<V>(
        self,
        values: impl IntoIterator<Item = T::Insert<V>>,
    ) -> TransactionBuilder<'tx, Tx, S, InsertBuilder<'a, S, InsertValuesSet, T>, InsertValuesSet>
    where
        T: SQLiteTable<'a>,
        T::Insert<V>: SQLModel<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.values(values);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

impl<'tx, 'a, Tx: ?Sized, S, T>
    TransactionBuilder<'tx, Tx, S, InsertBuilder<'a, S, InsertValuesSet, T>, InsertValuesSet>
where
    T: SQLiteTable<'a>,
{
    /// Begins a typed ON CONFLICT clause targeting a specific constraint.
    pub fn on_conflict<C: ConflictTarget<T>>(
        self,
        target: C,
    ) -> TransactionOnConflictBuilder<'tx, 'a, Tx, S, T> {
        TransactionOnConflictBuilder {
            transaction: self.transaction,
            builder: self.builder.on_conflict(target),
        }
    }

    /// Shorthand for `ON CONFLICT DO NOTHING` without specifying a target.
    pub fn on_conflict_do_nothing(
        self,
    ) -> TransactionBuilder<
        'tx,
        Tx,
        S,
        InsertBuilder<'a, S, InsertOnConflictSet, T>,
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
    ) -> InsertReturningTxBuilder<'tx, 'a, Tx, S, T, Columns>
    where
        Columns: ToSQL<'a, SQLiteValue<'a>> + IntoSelectTarget,
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

impl<'tx, 'a, Tx: ?Sized, S, T>
    TransactionBuilder<
        'tx,
        Tx,
        S,
        InsertBuilder<'a, S, InsertOnConflictSet, T>,
        InsertOnConflictSet,
    >
{
    /// Adds RETURNING clause after ON CONFLICT
    pub fn returning<Columns>(
        self,
        columns: Columns,
    ) -> InsertReturningTxBuilder<'tx, 'a, Tx, S, T, Columns>
    where
        Columns: ToSQL<'a, SQLiteValue<'a>> + IntoSelectTarget,
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

impl<'tx, 'a, Tx: ?Sized, S, T>
    TransactionBuilder<'tx, Tx, S, InsertBuilder<'a, S, InsertDoUpdateSet, T>, InsertDoUpdateSet>
{
    /// Adds WHERE clause after DO UPDATE SET
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> TransactionBuilder<
        'tx,
        Tx,
        S,
        InsertBuilder<'a, S, InsertOnConflictSet, T>,
        InsertOnConflictSet,
    >
    where
        E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
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
    ) -> InsertReturningTxBuilder<'tx, 'a, Tx, S, T, Columns>
    where
        Columns: ToSQL<'a, SQLiteValue<'a>> + IntoSelectTarget,
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
        T: ToSQL<'q, SQLiteValue<'q>>,
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
            E: drizzle_core::expr::Expr<'q, SQLiteValue<'q>>,
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
            Gr: drizzle_core::IntoGroupBy<'q, SQLiteValue<'q>>,
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
            E: drizzle_core::expr::Expr<'q, SQLiteValue<'q>>,
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
            TOrderBy: ToSQL<'q, SQLiteValue<'q>>,
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
}

impl_tx_select_methods! {
    SelectFromSet  => [r#where, group_by, order_by, limit, offset],
    SelectJoinSet  => [r#where, group_by, order_by],
    SelectWhereSet => [group_by, order_by, limit],
    SelectGroupSet => [having, order_by, limit],
    SelectOrderSet => [limit],
    SelectLimitSet => [offset],
    SelectSetOpSet => [order_by, limit, offset],
}

/// Emit a `<type>_join` method on `TransactionBuilder` that wraps the
/// corresponding `SelectBuilder<SelectFromSet | SelectJoinSet>` join method.
macro_rules! impl_tx_select_join {
    () => {
        impl_tx_select_join!(natural, drizzle_core::AfterJoin);
        impl_tx_select_join!(natural_left, drizzle_core::AfterLeftJoin);
        impl_tx_select_join!(left, drizzle_core::AfterLeftJoin);
        impl_tx_select_join!(left_outer, drizzle_core::AfterLeftJoin);
        impl_tx_select_join!(natural_left_outer, drizzle_core::AfterLeftJoin);
        impl_tx_select_join!(natural_right, drizzle_core::AfterRightJoin);
        impl_tx_select_join!(right, drizzle_core::AfterRightJoin);
        impl_tx_select_join!(right_outer, drizzle_core::AfterRightJoin);
        impl_tx_select_join!(natural_right_outer, drizzle_core::AfterRightJoin);
        impl_tx_select_join!(natural_full, drizzle_core::AfterFullJoin);
        impl_tx_select_join!(full, drizzle_core::AfterFullJoin);
        impl_tx_select_join!(full_outer, drizzle_core::AfterFullJoin);
        impl_tx_select_join!(natural_full_outer, drizzle_core::AfterFullJoin);
        impl_tx_select_join!(inner, drizzle_core::AfterJoin);
        impl_tx_select_join!(cross, drizzle_core::AfterJoin);
    };
    ($type:ident, $join_trait:path) => {
        paste::paste! {
            pub fn [<$type _join>]<J: JoinArg<'a, T>>(
                self,
                arg: J,
            ) -> TransactionBuilder<
                'tx,
                Tx,
                Schema,
                SelectBuilder<'a, Schema, SelectJoinSet, J::JoinedTable, <M as ScopePush<J::JoinedTable>>::Out, <M as $join_trait<R, J::JoinedTable>>::NewRow, G>,
                SelectJoinSet,
            >
            where
                M: $join_trait<R, J::JoinedTable> + ScopePush<J::JoinedTable>,
            {
                let builder = self.builder.[<$type _join>](arg);
                TransactionBuilder {
                    transaction: self.transaction,
                    builder,
                    _phantom: PhantomData,
                }
            }
        }
    };
}

impl<'tx, 'a, Tx: ?Sized, Schema, T, M, R, G>
    TransactionBuilder<
        'tx,
        Tx,
        Schema,
        SelectBuilder<'a, Schema, SelectFromSet, T, M, R, G>,
        SelectFromSet,
    >
{
    /// Plain `JOIN` — alias for `inner_join`.
    #[inline]
    pub fn join<J: JoinArg<'a, T>>(
        self,
        arg: J,
    ) -> TransactionBuilder<
        'tx,
        Tx,
        Schema,
        SelectBuilder<
            'a,
            Schema,
            SelectJoinSet,
            J::JoinedTable,
            <M as ScopePush<J::JoinedTable>>::Out,
            <M as drizzle_core::AfterJoin<R, J::JoinedTable>>::NewRow,
            G,
        >,
        SelectJoinSet,
    >
    where
        M: drizzle_core::AfterJoin<R, J::JoinedTable> + ScopePush<J::JoinedTable>,
    {
        let builder = self.builder.join(arg);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    impl_tx_select_join!();
}

impl<'tx, 'a, Tx: ?Sized, Schema, T, M, R, G>
    TransactionBuilder<
        'tx,
        Tx,
        Schema,
        SelectBuilder<'a, Schema, SelectJoinSet, T, M, R, G>,
        SelectJoinSet,
    >
{
    /// Plain `JOIN` — alias for `inner_join`. Chained-join form.
    #[inline]
    pub fn join<J: JoinArg<'a, T>>(
        self,
        arg: J,
    ) -> TransactionBuilder<
        'tx,
        Tx,
        Schema,
        SelectBuilder<
            'a,
            Schema,
            SelectJoinSet,
            J::JoinedTable,
            <M as ScopePush<J::JoinedTable>>::Out,
            <M as drizzle_core::AfterJoin<R, J::JoinedTable>>::NewRow,
            G,
        >,
        SelectJoinSet,
    >
    where
        M: drizzle_core::AfterJoin<R, J::JoinedTable> + ScopePush<J::JoinedTable>,
    {
        let builder = self.builder.join(arg);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    impl_tx_select_join!();
}

// -----------------------------------------------------------------------------
// Set operations (UNION / INTERSECT / EXCEPT, +`_all` variants).
// Anywhere a SelectBuilder is in an executable state, we can compose with
// another query of the same row marker / row type. The result lands in
// `SelectSetOpSet`, which itself accepts `order_by / limit / offset` (handled
// above by `impl_tx_select_methods!`).
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
// into_cte: lift a select into a CTE view (no `Tx` involvement — the
// resulting CTEView doesn't borrow the transaction).
// -----------------------------------------------------------------------------

impl<'tx, 'q, Tx: ?Sized, Schema, State, T, M, R, G>
    TransactionBuilder<'tx, Tx, Schema, SelectBuilder<'q, Schema, State, T, M, R, G>, State>
where
    State: AsCteState,
    T: drizzle_core::traits::SQLTable<
            'q,
            drizzle_sqlite::common::SQLiteSchemaType,
            SQLiteValue<'q>,
        >,
{
    #[inline]
    pub fn into_cte<Tag: drizzle_core::Tag + 'static>(
        self,
    ) -> CTEView<
        'q,
        <T as drizzle_core::traits::SQLTable<
            'q,
            drizzle_sqlite::common::SQLiteSchemaType,
            SQLiteValue<'q>,
        >>::Aliased<Tag>,
        SelectBuilder<'q, Schema, State, T, M, R, G>,
    > {
        self.builder.into_cte::<Tag>()
    }
}
