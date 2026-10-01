use crate::{common::MySQLSchemaType, helpers, values::MySQLValue};
use drizzle_core::{SQL, SQLTable, ToSQL, Token};

use super::ExecutableState;

pub use drizzle_core::builder::{
    SelectFromSet, SelectGroupSet, SelectInitial, SelectJoinSet, SelectLimitSet, SelectOffsetSet,
    SelectOrderSet, SelectSetOpSet, SelectWhereSet,
};

/// Marker for a SELECT after its single HAVING clause.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectHavingSet;

impl ExecutableState for SelectHavingSet {}

/// Marker for a base table that already carries one MySQL index hint.
///
/// MySQL rejects some mixed hint kinds for the same scope. Keeping the chosen
/// kind in the select state prevents an invalid second hint from being added.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectIndexHintSet<Kind>(core::marker::PhantomData<Kind>);

impl<Kind> ExecutableState for SelectIndexHintSet<Kind> {}

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

/// Clause gate for SELECT methods whose names collide with INSERT/UPDATE/DELETE
/// builder methods on the shared `QueryBuilder` type.
///
/// Coherence can only rule out overlapping inherent impls through a trait
/// local to this crate, so these clauses use this trait instead of
/// [`drizzle_core::ClauseAllowed`].
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "builder state `{Self}` does not allow `{C}`",
    label = "not available at this point of the query",
    note = "SELECT clauses go in order: FROM, JOIN, WHERE, GROUP BY, HAVING, ORDER BY, LIMIT, OFFSET",
    note = "only a SELECT can be a set operand, a subquery, a derived table, or an INSERT source"
)]
pub trait SelectClause<C> {}

/// MySQL clause marker: `OFFSET` (rendered with `LIMIT` when needed).
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct MySqlOffset;

// The shared SELECT states cover JOIN, GROUP BY, HAVING and the
// CTE/locking-read gate; MySQL adds its own states to those and keeps
// its own lists for clauses whose order rules differ.
impl SelectClause<drizzle_core::clause::Where> for SelectFromSet {}
impl<Kind> SelectClause<drizzle_core::clause::Where> for SelectIndexHintSet<Kind> {}
impl SelectClause<drizzle_core::clause::Where> for SelectJoinSet {}

impl SelectClause<drizzle_core::clause::OrderBy> for SelectFromSet {}
impl<Kind> SelectClause<drizzle_core::clause::OrderBy> for SelectIndexHintSet<Kind> {}
impl SelectClause<drizzle_core::clause::OrderBy> for SelectJoinSet {}
impl SelectClause<drizzle_core::clause::OrderBy> for SelectWhereSet {}
impl SelectClause<drizzle_core::clause::OrderBy> for SelectGroupSet {}
impl SelectClause<drizzle_core::clause::OrderBy> for SelectHavingSet {}

impl SelectClause<drizzle_core::clause::Limit> for SelectFromSet {}
impl<Kind> SelectClause<drizzle_core::clause::Limit> for SelectIndexHintSet<Kind> {}
impl SelectClause<drizzle_core::clause::Limit> for SelectJoinSet {}
impl SelectClause<drizzle_core::clause::Limit> for SelectWhereSet {}
impl SelectClause<drizzle_core::clause::Limit> for SelectGroupSet {}
impl SelectClause<drizzle_core::clause::Limit> for SelectHavingSet {}
impl SelectClause<drizzle_core::clause::Limit> for SelectOrderSet {}
impl SelectClause<drizzle_core::clause::Limit> for SelectSetOpSet {}

impl drizzle_core::ClauseAllowed<MySqlOffset> for SelectFromSet {}
impl<Kind> drizzle_core::ClauseAllowed<MySqlOffset> for SelectIndexHintSet<Kind> {}
impl drizzle_core::ClauseAllowed<MySqlOffset> for SelectJoinSet {}
impl drizzle_core::ClauseAllowed<MySqlOffset> for SelectWhereSet {}
impl drizzle_core::ClauseAllowed<MySqlOffset> for SelectGroupSet {}
impl drizzle_core::ClauseAllowed<MySqlOffset> for SelectHavingSet {}
impl drizzle_core::ClauseAllowed<MySqlOffset> for SelectOrderSet {}
impl drizzle_core::ClauseAllowed<MySqlOffset> for SelectSetOpSet {}

impl<Kind> drizzle_core::ClauseAllowed<drizzle_core::clause::Compound>
    for SelectIndexHintSet<Kind>
{
}
impl drizzle_core::ClauseAllowed<drizzle_core::clause::Compound> for SelectHavingSet {}

impl<Kind> drizzle_core::ClauseAllowed<drizzle_core::clause::Source> for SelectIndexHintSet<Kind> {}
impl drizzle_core::ClauseAllowed<drizzle_core::clause::Source> for SelectHavingSet {}
impl<Strength, Modifier> drizzle_core::ClauseAllowed<drizzle_core::clause::Source>
    for SelectForSet<Strength, Modifier>
{
}

impl<Kind> drizzle_core::ClauseAllowed<drizzle_core::clause::Join> for SelectIndexHintSet<Kind> {}

impl<Kind> drizzle_core::ClauseAllowed<drizzle_core::clause::GroupBy> for SelectIndexHintSet<Kind> {}

impl<Kind> drizzle_core::ClauseAllowed<drizzle_core::clause::Simple> for SelectIndexHintSet<Kind> {}
impl drizzle_core::ClauseAllowed<drizzle_core::clause::Simple> for SelectHavingSet {}

/// Typed MySQL `SELECT` builder.
pub type SelectBuilder<'a, Schema, State, Table = (), Marker = (), Row = (), Grouped = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row, Grouped>;

mod private {
    use super::{
        SelectForSet, SelectFromSet, SelectGroupSet, SelectHavingSet, SelectIndexHintSet,
        SelectJoinSet, SelectLimitSet, SelectOffsetSet, SelectOrderSet, SelectSetOpSet,
        SelectWhereSet,
    };

    pub trait SealedSelect {}

    pub trait Prepare {}

    impl Prepare for SelectFromSet {}
    impl<Kind> Prepare for SelectIndexHintSet<Kind> {}
    impl Prepare for SelectJoinSet {}
    impl Prepare for SelectWhereSet {}
    impl Prepare for SelectGroupSet {}
    impl Prepare for SelectHavingSet {}
    impl Prepare for SelectOrderSet {}
    impl Prepare for SelectLimitSet {}
    impl Prepare for SelectOffsetSet {}
    impl Prepare for SelectSetOpSet {}
    impl<Strength, Modifier> Prepare for SelectForSet<Strength, Modifier> {}
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: private::Prepare,
{
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

impl<'a, S, M> SelectBuilder<'a, S, SelectInitial, (), M> {
    /// Sets the source selected from by this query.
    #[allow(clippy::type_complexity)]
    pub fn from<T>(
        self,
        table: T,
    ) -> SelectBuilder<
        'a,
        S,
        SelectFromSet,
        T,
        drizzle_core::FromMarker<M, T>,
        <M as drizzle_core::ResolveRow<T>>::Row,
    >
    where
        T: ToSQL<'a, MySQLValue<'a>> + drizzle_core::ScopeEntry,
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
    pub fn use_index<Indexes>(
        self,
        indexes: Indexes,
    ) -> SelectBuilder<'a, S, SelectIndexHintSet<helpers::UseIndex>, T, M, R, G>
    where
        Indexes: helpers::IndexHintList<'a, T>,
    {
        SelectBuilder::from_sql(self.sql.append(
            helpers::index_hint::<T, Indexes, helpers::UseIndex>(&indexes),
        ))
    }

    /// Advises MySQL to strongly prefer this table's generated index.
    #[must_use]
    pub fn force_index<Indexes>(
        self,
        indexes: Indexes,
    ) -> SelectBuilder<'a, S, SelectIndexHintSet<helpers::ForceIndex>, T, M, R, G>
    where
        Indexes: helpers::IndexHintList<'a, T>,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::index_hint::<
            T,
            Indexes,
            helpers::ForceIndex,
        >(&indexes)))
    }

    /// Advises MySQL not to use this table's generated index.
    #[must_use]
    pub fn ignore_index<Indexes>(
        self,
        indexes: Indexes,
    ) -> SelectBuilder<'a, S, SelectIndexHintSet<helpers::IgnoreIndex>, T, M, R, G>
    where
        Indexes: helpers::IndexHintList<'a, T>,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::index_hint::<
            T,
            Indexes,
            helpers::IgnoreIndex,
        >(&indexes)))
    }
}

macro_rules! join_on_method {
    ($name:ident, $join:expr, $kind:ident) => {
        #[doc = concat!("Adds a typed `", stringify!($name), "` join.")]
        #[allow(clippy::type_complexity)]
        pub fn $name<J: helpers::JoinArg<'a, T>>(
            self,
            arg: J,
        ) -> SelectBuilder<
            'a,
            S,
            SelectJoinSet,
            J::JoinedTable,
            <M as drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::$kind, J::OnSources>>::Marker,
            <M as drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::$kind, J::OnSources>>::Row,
            G,
        >
        where
            M: drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::$kind, J::OnSources>,
        {
            SelectBuilder::from_sql(self.sql.append(arg.into_join_sql($join)))
        }
    };
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Join>,
{
    join_on_method!(join, drizzle_core::Join::new(), InnerJoin);
    join_on_method!(inner_join, drizzle_core::Join::new().inner(), InnerJoin);
    join_on_method!(left_join, drizzle_core::Join::new().left(), LeftJoin);
    join_on_method!(
        left_outer_join,
        drizzle_core::Join::new().left().outer(),
        LeftJoin
    );
    join_on_method!(right_join, drizzle_core::Join::new().right(), RightJoin);
    join_on_method!(
        right_outer_join,
        drizzle_core::Join::new().right().outer(),
        RightJoin
    );

    /// Adds a cross join.
    ///
    /// A bare source renders `CROSS JOIN`. For backwards compatibility,
    /// `(source, predicate)` renders the equivalent `INNER JOIN ... ON ...`.
    #[allow(clippy::type_complexity)]
    pub fn cross_join<Arg: helpers::CrossJoinArg<'a, T>>(
        self,
        arg: Arg,
    ) -> SelectBuilder<
        'a,
        S,
        SelectJoinSet,
        Arg::JoinedTable,
        <M as drizzle_core::JoinStep<
            R,
            Arg::JoinedTable,
            drizzle_core::InnerJoin,
            Arg::OnSources,
        >>::Marker,
        <M as drizzle_core::JoinStep<
            R,
            Arg::JoinedTable,
            drizzle_core::InnerJoin,
            Arg::OnSources,
        >>::Row,
        G,
    >
    where
        M: drizzle_core::JoinStep<R, Arg::JoinedTable, drizzle_core::InnerJoin, Arg::OnSources>,
    {
        SelectBuilder::from_sql(self.sql.append(arg.into_cross_join_sql()))
    }

    /// Adds an INNER JOIN LATERAL clause.
    #[allow(clippy::type_complexity)]
    pub fn inner_join_lateral<Arg>(
        self,
        arg: Arg,
    ) -> SelectBuilder<
        'a,
        S,
        SelectJoinSet,
        Arg::JoinedTable,
        <M as drizzle_core::JoinStep<
            R,
            Arg::JoinedTable,
            drizzle_core::Lateral<drizzle_core::InnerJoin>,
            Arg::OnSources,
        >>::Marker,
        <M as drizzle_core::JoinStep<
            R,
            Arg::JoinedTable,
            drizzle_core::Lateral<drizzle_core::InnerJoin>,
            Arg::OnSources,
        >>::Row,
        G,
    >
    where
        Arg: drizzle_core::LateralArg<'a, MySQLValue<'a>>,
        M: drizzle_core::JoinStep<
                R,
                Arg::JoinedTable,
                drizzle_core::Lateral<drizzle_core::InnerJoin>,
                Arg::OnSources,
            >,
    {
        SelectBuilder::from_sql(
            self.sql
                .append(arg.into_lateral_sql(drizzle_core::Join::new().inner())),
        )
    }

    /// Adds a LEFT JOIN LATERAL clause.
    #[allow(clippy::type_complexity)]
    pub fn left_join_lateral<Arg, SelectionProof>(
        self,
        arg: Arg,
    ) -> SelectBuilder<
        'a,
        S,
        SelectJoinSet,
        Arg::JoinedTable,
        <M as drizzle_core::JoinStep<
            R,
            Arg::JoinedTable,
            drizzle_core::Lateral<drizzle_core::LeftJoin>,
            Arg::OnSources,
        >>::Marker,
        <M as drizzle_core::JoinStep<
            R,
            Arg::JoinedTable,
            drizzle_core::Lateral<drizzle_core::LeftJoin>,
            Arg::OnSources,
        >>::Row,
        G,
    >
    where
        Arg: drizzle_core::LateralArg<'a, MySQLValue<'a>>,
        M: drizzle_core::JoinStep<
                R,
                Arg::JoinedTable,
                drizzle_core::Lateral<drizzle_core::LeftJoin>,
                Arg::OnSources,
            > + drizzle_core::LeftLateralSelection<SelectionProof>,
    {
        SelectBuilder::from_sql(
            self.sql
                .append(arg.into_lateral_sql(drizzle_core::Join::new().left())),
        )
    }

    /// Adds a CROSS JOIN LATERAL clause without an ON condition.
    #[allow(clippy::type_complexity)]
    pub fn cross_join_lateral<Source>(
        self,
        source: Source,
    ) -> SelectBuilder<
        'a,
        S,
        SelectJoinSet,
        Source::JoinedTable,
        <M as drizzle_core::JoinStep<
            R,
            Source::JoinedTable,
            drizzle_core::Lateral<drizzle_core::InnerJoin>,
        >>::Marker,
        <M as drizzle_core::JoinStep<
            R,
            Source::JoinedTable,
            drizzle_core::Lateral<drizzle_core::InnerJoin>,
        >>::Row,
        G,
    >
    where
        Source: drizzle_core::LateralSource<'a, MySQLValue<'a>>,
        M: drizzle_core::JoinStep<
                R,
                Source::JoinedTable,
                drizzle_core::Lateral<drizzle_core::InnerJoin>,
            >,
    {
        SelectBuilder::from_sql(self.sql.append(source.into_cross_lateral_sql()))
    }
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: SelectClause<drizzle_core::clause::Where>,
{
    /// Filters rows before grouping and projection.
    #[allow(clippy::type_complexity)]
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> SelectBuilder<
        'a,
        S,
        SelectWhereSet,
        T,
        <M as drizzle_core::HasScope>::With<E::Sources>,
        R,
        G,
    >
    where
        M: drizzle_core::HasScope,
        E: drizzle_core::expr::Expr<'a, MySQLValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::r#where(condition)))
    }
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::GroupBy>,
{
    /// Groups rows by the supplied expressions.
    #[allow(clippy::type_complexity)]
    pub fn group_by<Gr>(
        self,
        columns: Gr,
    ) -> SelectBuilder<
        'a,
        S,
        SelectGroupSet,
        T,
        <M as drizzle_core::HasScope>::With<Gr::Sources>,
        R,
        Gr::Columns,
    >
    where
        M: drizzle_core::HasScope,
        Gr: drizzle_core::IntoGroupBy<'a, MySQLValue<'a>>,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::group_by_expr(columns)))
    }
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Having>,
{
    /// Filters grouped rows.
    #[allow(clippy::type_complexity)]
    pub fn having<E>(
        self,
        condition: E,
    ) -> SelectBuilder<
        'a,
        S,
        SelectHavingSet,
        T,
        <M as drizzle_core::HasScope>::With<E::Sources>,
        R,
        G,
    >
    where
        M: drizzle_core::HasScope,
        E: drizzle_core::expr::Expr<'a, MySQLValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::having(condition)))
    }
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: SelectClause<drizzle_core::clause::OrderBy>,
{
    /// Orders the selected rows.
    #[allow(clippy::type_complexity)]
    pub fn order_by<O>(
        self,
        order: O,
    ) -> SelectBuilder<
        'a,
        S,
        SelectOrderSet,
        T,
        <M as drizzle_core::HasScope>::With<O::Sources>,
        R,
        G,
    >
    where
        M: drizzle_core::HasScope,
        O: ToSQL<'a, MySQLValue<'a>> + drizzle_core::expr::ExprSources,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::order_by(order)))
    }
}

impl<'a, S, T, M, R, G> SelectBuilder<'a, S, SelectSetOpSet, T, M, R, G> {
    /// Orders a compound query by an output expression or alias.
    pub fn order_by<O, Proof>(self, order: O) -> SelectBuilder<'a, S, SelectOrderSet, T, M, R, G>
    where
        O: helpers::SetOrderBy<'a, M, T, Proof>,
    {
        let order = helpers::SetOrderBy::into_set_order_sql(order);
        SelectBuilder::from_sql(
            self.sql
                .append(SQL::from_iter([Token::ORDER, Token::BY]).append(order)),
        )
    }
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: SelectClause<drizzle_core::clause::Limit>,
{
    /// Limits the number of returned rows.
    #[track_caller]
    pub fn limit<P>(self, limit: P) -> SelectBuilder<'a, S, SelectLimitSet, T, M, R, G>
    where
        P: drizzle_core::PaginationArg<'a, MySQLValue<'a>>,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::limit(limit)))
    }
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<MySqlOffset>,
{
    /// Skips rows without an explicit limit.
    ///
    /// MySQL has no bare `OFFSET`, so this renders `LIMIT 9223372036854775807`
    /// before the offset (see [`drizzle_core::helpers::MYSQL_UNBOUNDED_LIMIT`]).
    #[track_caller]
    pub fn offset<P>(self, offset: P) -> SelectBuilder<'a, S, SelectOffsetSet, T, M, R, G>
    where
        P: drizzle_core::PaginationArg<'a, MySQLValue<'a>>,
    {
        SelectBuilder::from_sql(self.sql.append(helpers::standalone_offset(offset)))
    }
}

impl<'a, S, T, M, R, G> SelectBuilder<'a, S, SelectLimitSet, T, M, R, G> {
    /// Skips rows after an explicit limit.
    #[track_caller]
    pub fn offset<P>(self, offset: P) -> SelectBuilder<'a, S, SelectOffsetSet, T, M, R, G>
    where
        P: drizzle_core::PaginationArg<'a, MySQLValue<'a>>,
    {
        SelectBuilder::from_sql(self.sql.append(drizzle_core::helpers::offset(offset)))
    }
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Simple>,
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
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Simple> + ExecutableState,
    T: SQLTable<'a, MySQLSchemaType, MySQLValue<'a>>,
{
    /// Converts this query into a named common table expression.
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

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Source>,
{
    /// Names this completed projection for use as a derived table.
    ///
    /// # Panics
    ///
    /// Panics when the projection contains duplicate output names. Name a
    /// computed expression with [`drizzle_core::expr::AliasExt::named`] to
    /// make each output unique.
    #[must_use]
    pub fn alias<Tag, AggProof>(
        self,
        _tag: Tag,
    ) -> drizzle_core::Derived<
        'a,
        MySQLValue<'a>,
        Tag,
        <M as drizzle_core::DerivedSelection<'a, MySQLValue<'a>, MySQLSchemaType, T>>::Projection,
        Self,
    >
    where
        Tag: drizzle_core::Tag,
        M: drizzle_core::DerivedSelection<'a, MySQLValue<'a>, MySQLSchemaType, T>
            + drizzle_core::row::MarkerAggValidFor<G, AggProof>,
        <M as drizzle_core::DerivedSelection<'a, MySQLValue<'a>, MySQLSchemaType, T>>::Projection:
            drizzle_core::DerivedProjection<Tag>,
    {
        // SAFETY: The executable-state, aggregate, and projection bounds
        // above prove that this query matches the derived projection; its
        // scope travels in `Self`'s sources and is checked where it is used.
        unsafe { drizzle_core::Derived::new_unchecked(self) }
    }
}

macro_rules! set_operation {
    ($name:ident, $token:expr, $all:expr) => {
        #[doc = concat!("Combines this query with another using `", stringify!($name), "`.")]
        #[allow(clippy::type_complexity)]
        pub fn $name<O>(
            self,
            other: O,
        ) -> SelectBuilder<
            'a,
            S,
            SelectSetOpSet,
            T,
            <M as drizzle_core::SetOperand<O::Marker>>::Combined,
            R,
            G,
        >
        where
            O: IntoSelectQuery<'a, S, R>,
            M: drizzle_core::SetOperand<O::Marker>,
        {
            SelectBuilder::from_sql(helpers::set_op(
                self.sql,
                $token,
                $all,
                other.into_select_query().into_select_sql(),
            ))
        }
    };
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Compound>,
{
    set_operation!(union, drizzle_core::Token::UNION, false);
    set_operation!(union_all, drizzle_core::Token::UNION, true);
    set_operation!(intersect, drizzle_core::Token::INTERSECT, false);
    set_operation!(intersect_all, drizzle_core::Token::INTERSECT, true);
    set_operation!(except, drizzle_core::Token::EXCEPT, false);
    set_operation!(except_all, drizzle_core::Token::EXCEPT, true);
}

/// A completed SELECT with the inferred row shape `R`.
///
/// This trait is sealed so INSERT ... SELECT and set operations cannot accept
/// arbitrary SQL or DML builders.
#[doc(hidden)]
pub trait CompletedSelect<'a, S, R>: private::SealedSelect {
    type Marker;
    type Grouped;

    fn into_select_sql(self) -> drizzle_core::SQL<'a, MySQLValue<'a>>;
}

/// Safe extension seam for driver wrappers around a completed MySQL select.
///
/// Implementations must unwrap to the sealed [`CompletedSelect`] type; they cannot
/// manufacture an arbitrary SQL fragment or row marker.
#[doc(hidden)]
pub trait IntoSelectQuery<'a, S, R> {
    type Marker;
    type Grouped;
    type Select: CompletedSelect<'a, S, R, Marker = Self::Marker, Grouped = Self::Grouped>;

    fn into_select_query(self) -> Self::Select;
}

impl<'a, S, State, T, M, R, G> private::SealedSelect for SelectBuilder<'a, S, State, T, M, R, G> where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Compound>
{
}

impl<'a, S, State, T, M, R, G> CompletedSelect<'a, S, R> for SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Compound>,
{
    type Marker = M;
    type Grouped = G;

    fn into_select_sql(self) -> drizzle_core::SQL<'a, MySQLValue<'a>> {
        self.sql
    }
}

impl<'a, S, State, T, M, R, G> IntoSelectQuery<'a, S, R> for SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Compound>,
{
    type Marker = M;
    type Grouped = G;
    type Select = Self;

    fn into_select_query(self) -> Self::Select {
        self
    }
}

impl<'a, S, State, T, M, R, G> drizzle_core::expr::Expr<'a, MySQLValue<'a>>
    for SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Compound> + ExecutableState,
    M: drizzle_core::expr::SubqueryType<'a, MySQLValue<'a>> + drizzle_core::SelectSources,
{
    type SQLType = <M as drizzle_core::expr::SubqueryType<'a, MySQLValue<'a>>>::SQLType;
    type Nullable = drizzle_core::expr::Null;
    type Aggregate = drizzle_core::expr::Scalar;
}

impl<S, State: drizzle_core::ClauseAllowed<drizzle_core::clause::Source>, T, M, R, G>
    drizzle_core::expr::SelectQuery for SelectBuilder<'_, S, State, T, M, R, G>
{
}

impl<S, State, T, M, R, G> drizzle_core::expr::ExprSources
    for SelectBuilder<'_, S, State, T, M, R, G>
where
    M: drizzle_core::SelectSources,
{
    type Sources = M::Sources;
}
