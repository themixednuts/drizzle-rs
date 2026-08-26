use crate::{common::MySQLSchemaType, helpers, values::MySQLValue};
use drizzle_core::{SQLTable, ToSQL};

use super::ExecutableState;

pub use drizzle_core::builder::{
    AsCteState, SelectFromSet, SelectGroupSet, SelectInitial, SelectJoinSet, SelectLimitSet,
    SelectOffsetSet, SelectOrderSet, SelectSetOpSet, SelectWhereSet,
};

/// Marker for a SELECT after its single HAVING clause.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectHavingSet;

impl ExecutableState for SelectHavingSet {}
impl drizzle_core::GroupByApplied for SelectHavingSet {}
impl AsCteState for SelectHavingSet {}

/// Marker for a base table that already carries one MySQL index hint.
///
/// MySQL rejects some mixed hint kinds for the same scope. Keeping the chosen
/// kind in the select state prevents an invalid second hint from being added.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectIndexHintSet<Kind>(core::marker::PhantomData<Kind>);

impl<Kind> ExecutableState for SelectIndexHintSet<Kind> {}
impl<Kind> AsCteState for SelectIndexHintSet<Kind> {}
impl<Kind> drizzle_core::JoinAllowed for SelectIndexHintSet<Kind> {}
impl<Kind> drizzle_core::GroupByAllowed for SelectIndexHintSet<Kind> {}

/// Marker for the MySQL `FOR UPDATE` lock strength.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ForUpdate;

/// Marker for the MySQL `FOR SHARE` lock strength.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ForShare;

/// Marker for a locking read without a wait modifier.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Wait;

/// Marker for a locking read with `NOWAIT`.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NoWait;

/// Marker for a locking read with `SKIP LOCKED`.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SkipLocked;

/// Terminal MySQL locking-read state.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectForSet<Strength, Modifier = Wait>(core::marker::PhantomData<(Strength, Modifier)>);

impl<Strength, Modifier> ExecutableState for SelectForSet<Strength, Modifier> {}

#[doc(hidden)]
pub trait SelectWhereAllowed {}
impl SelectWhereAllowed for SelectFromSet {}
impl<Kind> SelectWhereAllowed for SelectIndexHintSet<Kind> {}
impl SelectWhereAllowed for SelectJoinSet {}

#[doc(hidden)]
pub trait SelectLimitAllowed {}
impl SelectLimitAllowed for SelectFromSet {}
impl<Kind> SelectLimitAllowed for SelectIndexHintSet<Kind> {}
impl SelectLimitAllowed for SelectJoinSet {}
impl SelectLimitAllowed for SelectWhereSet {}
impl SelectLimitAllowed for SelectGroupSet {}
impl SelectLimitAllowed for SelectHavingSet {}
impl SelectLimitAllowed for SelectOrderSet {}
impl SelectLimitAllowed for SelectSetOpSet {}

pub type SelectBuilder<'a, Schema, State, Table = (), Marker = (), Row = (), Grouped = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row, Grouped>;

macro_rules! select_prepare {
    ($state:ty $(, $extra:ident)*) => {
        impl<'a, S, T, M, R, G, $($extra),*> SelectBuilder<'a, S, $state, T, M, R, G> {
            /// Compiles named or anonymous placeholders into MySQL's ordered
            /// positional bind plan after validating scope and grouping.
            #[must_use]
            pub fn prepare<ScopeProof, AggProof>(
                &self,
            ) -> drizzle_core::prepared::PreparedStatement<'a, MySQLValue<'a>>
            where
                M: drizzle_core::row::MarkerScopeValidFor<ScopeProof>
                    + drizzle_core::row::MarkerAggValidFor<G, AggProof>,
            {
                self.prepared_statement()
            }
        }
    };
}

select_prepare!(SelectFromSet);
select_prepare!(SelectIndexHintSet<Kind>, Kind);
select_prepare!(SelectJoinSet);
select_prepare!(SelectWhereSet);
select_prepare!(SelectGroupSet);
select_prepare!(SelectHavingSet);
select_prepare!(SelectOrderSet);
select_prepare!(SelectLimitSet);
select_prepare!(SelectOffsetSet);
select_prepare!(SelectSetOpSet);

impl<'a, S, T, M, R, G, Strength, Modifier>
    SelectBuilder<'a, S, SelectForSet<Strength, Modifier>, T, M, R, G>
{
    /// Compiles a locking read into MySQL's ordered positional bind plan.
    #[must_use]
    pub fn prepare<ScopeProof, AggProof>(
        &self,
    ) -> drizzle_core::prepared::PreparedStatement<'a, MySQLValue<'a>>
    where
        M: drizzle_core::row::MarkerScopeValidFor<ScopeProof>
            + drizzle_core::row::MarkerAggValidFor<G, AggProof>,
    {
        self.prepared_statement()
    }
}

impl<'a, S, M> SelectBuilder<'a, S, SelectInitial, (), M> {
    #[allow(clippy::type_complexity)]
    pub fn from<T>(
        self,
        table: T,
    ) -> SelectBuilder<
        'a,
        S,
        SelectFromSet,
        T,
        drizzle_core::Scoped<M, drizzle_core::Cons<T, drizzle_core::Nil>>,
        <M as drizzle_core::ResolveRow<T>>::Row,
    >
    where
        T: ToSQL<'a, MySQLValue<'a>>,
        M: drizzle_core::ResolveRow<T>,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::from(table)))
    }
}

impl<'a, S, T, M, R, G> SelectBuilder<'a, S, SelectFromSet, T, M, R, G>
where
    T: crate::traits::MySQLTable<'a>,
{
    /// Advises MySQL to consider this table's generated index.
    #[must_use]
    pub fn use_index<Index>(
        self,
        index: Index,
    ) -> SelectBuilder<'a, S, SelectIndexHintSet<helpers::UseIndex>, T, M, R, G>
    where
        Index: drizzle_core::SQLIndex<'a, MySQLSchemaType, MySQLValue<'a>, Table = T>,
    {
        SelectBuilder::from_sql(
            self.sql
                .append(helpers::index_hint::<T, Index, helpers::UseIndex>(&index)),
        )
    }

    /// Advises MySQL to strongly prefer this table's generated index.
    #[must_use]
    pub fn force_index<Index>(
        self,
        index: Index,
    ) -> SelectBuilder<'a, S, SelectIndexHintSet<helpers::ForceIndex>, T, M, R, G>
    where
        Index: drizzle_core::SQLIndex<'a, MySQLSchemaType, MySQLValue<'a>, Table = T>,
    {
        SelectBuilder::from_sql(
            self.sql
                .append(helpers::index_hint::<T, Index, helpers::ForceIndex>(&index)),
        )
    }

    /// Advises MySQL not to use this table's generated index.
    #[must_use]
    pub fn ignore_index<Index>(
        self,
        index: Index,
    ) -> SelectBuilder<'a, S, SelectIndexHintSet<helpers::IgnoreIndex>, T, M, R, G>
    where
        Index: drizzle_core::SQLIndex<'a, MySQLSchemaType, MySQLValue<'a>, Table = T>,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::index_hint::<
            T,
            Index,
            helpers::IgnoreIndex,
        >(&index)))
    }
}

macro_rules! join_on_method {
    ($name:ident, $join:expr, $row_trait:ident) => {
        #[allow(clippy::type_complexity)]
        pub fn $name<J: helpers::JoinArg<'a, T>>(
            self,
            arg: J,
        ) -> SelectBuilder<
            'a,
            S,
            SelectJoinSet,
            J::JoinedTable,
            <M as drizzle_core::ScopePush<J::JoinedTable>>::Out,
            <M as drizzle_core::$row_trait<R, J::JoinedTable>>::NewRow,
            G,
        >
        where
            M: drizzle_core::$row_trait<R, J::JoinedTable>
                + drizzle_core::ScopePush<J::JoinedTable>,
        {
            SelectBuilder::from_sql(self.sql.append(arg.into_join_sql($join)))
        }
    };
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::JoinAllowed,
{
    join_on_method!(join, drizzle_core::Join::new(), AfterJoin);
    join_on_method!(inner_join, drizzle_core::Join::new().inner(), AfterJoin);
    join_on_method!(cross_join, drizzle_core::Join::new().cross(), AfterJoin);
    join_on_method!(left_join, drizzle_core::Join::new().left(), AfterLeftJoin);
    join_on_method!(
        left_outer_join,
        drizzle_core::Join::new().left().outer(),
        AfterLeftJoin
    );
    join_on_method!(
        right_join,
        drizzle_core::Join::new().right(),
        AfterRightJoin
    );
    join_on_method!(
        right_outer_join,
        drizzle_core::Join::new().right().outer(),
        AfterRightJoin
    );
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: SelectWhereAllowed,
{
    pub fn r#where<E>(self, condition: E) -> SelectBuilder<'a, S, SelectWhereSet, T, M, R, G>
    where
        E: drizzle_core::expr::Expr<'a, MySQLValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::r#where(condition)))
    }
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::GroupByAllowed,
{
    pub fn group_by<Gr>(
        self,
        columns: Gr,
    ) -> SelectBuilder<'a, S, SelectGroupSet, T, M, R, Gr::Columns>
    where
        Gr: drizzle_core::IntoGroupBy<'a, MySQLValue<'a>>,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::group_by_expr(columns)))
    }
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::HavingAllowed,
{
    pub fn having<E>(self, condition: E) -> SelectBuilder<'a, S, SelectHavingSet, T, M, R, G>
    where
        E: drizzle_core::expr::Expr<'a, MySQLValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::having(condition)))
    }
}

macro_rules! select_order_by {
    ($state:ty $(, $extra:ident)*) => {
        impl<'a, S, T, M, R, G, $($extra),*> SelectBuilder<'a, S, $state, T, M, R, G> {
            pub fn order_by<O>(self, order: O) -> SelectBuilder<'a, S, SelectOrderSet, T, M, R, G>
            where
                O: ToSQL<'a, MySQLValue<'a>>,
            {
                SelectBuilder::from_sql(self.sql.append(helpers::order_by(order)))
            }
        }
    };
}

select_order_by!(SelectFromSet);
select_order_by!(SelectIndexHintSet<Kind>, Kind);
select_order_by!(SelectJoinSet);
select_order_by!(SelectWhereSet);
select_order_by!(SelectGroupSet);
select_order_by!(SelectHavingSet);

impl<'a, S, T, M, R, G> SelectBuilder<'a, S, SelectSetOpSet, T, M, R, G> {
    pub fn order_by<O, Proof>(self, order: O) -> SelectBuilder<'a, S, SelectOrderSet, T, M, R, G>
    where
        O: helpers::SetOrderBy<'a, M, T, Proof>,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::set_order_by::<M, T, Proof>(order)))
    }
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: SelectLimitAllowed,
{
    #[track_caller]
    pub fn limit<P>(self, limit: P) -> SelectBuilder<'a, S, SelectLimitSet, T, M, R, G>
    where
        P: drizzle_core::PaginationArg<'a, MySQLValue<'a>>,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::limit(limit)))
    }
}

macro_rules! standalone_offset {
    ($state:ty $(, $extra:ident)*) => {
        impl<'a, S, T, M, R, G, $($extra),*> SelectBuilder<'a, S, $state, T, M, R, G> {
            #[track_caller]
            pub fn offset<P>(self, offset: P) -> SelectBuilder<'a, S, SelectOffsetSet, T, M, R, G>
            where
                P: drizzle_core::PaginationArg<'a, MySQLValue<'a>>,
            {
                SelectBuilder::from_sql(self.sql.append(helpers::standalone_offset(offset)))
            }
        }
    };
}

standalone_offset!(SelectFromSet);
standalone_offset!(SelectIndexHintSet<Kind>, Kind);
standalone_offset!(SelectJoinSet);
standalone_offset!(SelectWhereSet);
standalone_offset!(SelectGroupSet);
standalone_offset!(SelectHavingSet);
standalone_offset!(SelectOrderSet);
standalone_offset!(SelectSetOpSet);

impl<'a, S, T, M, R, G> SelectBuilder<'a, S, SelectLimitSet, T, M, R, G> {
    #[track_caller]
    pub fn offset<P>(self, offset: P) -> SelectBuilder<'a, S, SelectOffsetSet, T, M, R, G>
    where
        P: drizzle_core::PaginationArg<'a, MySQLValue<'a>>,
    {
        SelectBuilder::from_sql(self.sql.append(drizzle_core::helpers::offset(offset)))
    }
}

/// Select states on which MySQL permits a terminal locking clause.
#[doc(hidden)]
pub trait LockingReadAllowed {}

impl LockingReadAllowed for SelectFromSet {}
impl<Kind> LockingReadAllowed for SelectIndexHintSet<Kind> {}
impl LockingReadAllowed for SelectJoinSet {}
impl LockingReadAllowed for SelectWhereSet {}
impl LockingReadAllowed for SelectGroupSet {}
impl LockingReadAllowed for SelectHavingSet {}
impl LockingReadAllowed for SelectOrderSet {}
impl LockingReadAllowed for SelectLimitSet {}
impl LockingReadAllowed for SelectOffsetSet {}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: LockingReadAllowed,
{
    /// Locks matching rows for update.
    #[must_use]
    pub fn for_update(self) -> SelectBuilder<'a, S, SelectForSet<ForUpdate>, T, M, R, G> {
        SelectBuilder::from_sql(
            self.sql
                .push(drizzle_core::Token::FOR)
                .push(drizzle_core::Token::UPDATE),
        )
    }

    /// Acquires shared locks on matching rows.
    #[must_use]
    pub fn for_share(self) -> SelectBuilder<'a, S, SelectForSet<ForShare>, T, M, R, G> {
        SelectBuilder::from_sql(
            self.sql
                .push(drizzle_core::Token::FOR)
                .push(drizzle_core::Token::SHARE),
        )
    }
}

impl<'a, S, Strength, T, M, R, G> SelectBuilder<'a, S, SelectForSet<Strength, Wait>, T, M, R, G> {
    /// Fails immediately instead of waiting for a conflicting row lock.
    #[must_use]
    pub fn nowait(self) -> SelectBuilder<'a, S, SelectForSet<Strength, NoWait>, T, M, R, G> {
        SelectBuilder::from_sql(self.sql.push(drizzle_core::Token::NOWAIT))
    }

    /// Skips rows currently held by another transaction.
    #[must_use]
    pub fn skip_locked(
        self,
    ) -> SelectBuilder<'a, S, SelectForSet<Strength, SkipLocked>, T, M, R, G> {
        SelectBuilder::from_sql(
            self.sql
                .push(drizzle_core::Token::SKIP)
                .push(drizzle_core::Token::LOCKED),
        )
    }
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: AsCteState + ExecutableState,
    T: SQLTable<'a, MySQLSchemaType, MySQLValue<'a>>,
{
    #[must_use]
    pub fn into_cte<Tag: drizzle_core::Tag + 'static>(
        self,
    ) -> super::CTEView<'a, <T as SQLTable<'a, MySQLSchemaType, MySQLValue<'a>>>::Aliased<Tag>, Self>
    {
        super::CTEView::new(
            <T as SQLTable<'a, MySQLSchemaType, MySQLValue<'a>>>::alias::<Tag>(),
            Tag::NAME,
            self,
        )
    }
}

macro_rules! set_operation {
    ($name:ident, $token:expr, $all:expr) => {
        pub fn $name(
            self,
            other: impl IntoSelect<'a, S, R>,
        ) -> SelectBuilder<'a, S, SelectSetOpSet, T, M, R, G> {
            SelectBuilder::from_sql(helpers::set_op(
                self.sql,
                $token,
                $all,
                other.into_select_sql(),
            ))
        }
    };
}

macro_rules! impl_set_operations {
    ($state:ty) => {
        impl<'a, S, T, M, R, G> SelectBuilder<'a, S, $state, T, M, R, G> {
            set_operation!(union, drizzle_core::Token::UNION, false);
            set_operation!(union_all, drizzle_core::Token::UNION, true);
            set_operation!(intersect, drizzle_core::Token::INTERSECT, false);
            set_operation!(intersect_all, drizzle_core::Token::INTERSECT, true);
            set_operation!(except, drizzle_core::Token::EXCEPT, false);
            set_operation!(except_all, drizzle_core::Token::EXCEPT, true);
        }
    };
}

impl_set_operations!(SelectFromSet);
impl_set_operations!(SelectJoinSet);
impl_set_operations!(SelectWhereSet);
impl_set_operations!(SelectGroupSet);
impl_set_operations!(SelectHavingSet);
impl_set_operations!(SelectOrderSet);
impl_set_operations!(SelectLimitSet);
impl_set_operations!(SelectOffsetSet);
impl_set_operations!(SelectSetOpSet);

mod private {
    pub trait SealedSelect {}
}

/// A completed SELECT with the inferred row shape `R`.
///
/// This trait is sealed so INSERT ... SELECT and set operations cannot accept
/// arbitrary SQL or DML builders.
#[doc(hidden)]
pub trait IntoSelect<'a, S, R>: private::SealedSelect {
    type Marker;

    fn into_select_sql(self) -> drizzle_core::SQL<'a, MySQLValue<'a>>;
}

macro_rules! impl_completed_select {
    ($state:ty) => {
        impl<'a, S, T, M, R, G> private::SealedSelect for SelectBuilder<'a, S, $state, T, M, R, G> {}

        impl<'a, S, T, M, R, G> IntoSelect<'a, S, R> for SelectBuilder<'a, S, $state, T, M, R, G> {
            type Marker = M;

            fn into_select_sql(self) -> drizzle_core::SQL<'a, MySQLValue<'a>> {
                self.sql
            }
        }

        impl<'a, S, T, M, R, G> drizzle_core::expr::Expr<'a, MySQLValue<'a>>
            for SelectBuilder<'a, S, $state, T, M, R, G>
        where
            M: drizzle_core::expr::SubqueryType<'a, MySQLValue<'a>>,
        {
            type SQLType = <M as drizzle_core::expr::SubqueryType<'a, MySQLValue<'a>>>::SQLType;
            type Nullable = drizzle_core::expr::Null;
            type Aggregate = drizzle_core::expr::Scalar;
        }
    };
}

impl_completed_select!(SelectFromSet);
impl_completed_select!(SelectJoinSet);
impl_completed_select!(SelectWhereSet);
impl_completed_select!(SelectGroupSet);
impl_completed_select!(SelectHavingSet);
impl_completed_select!(SelectOrderSet);
impl_completed_select!(SelectLimitSet);
impl_completed_select!(SelectOffsetSet);
impl_completed_select!(SelectSetOpSet);
