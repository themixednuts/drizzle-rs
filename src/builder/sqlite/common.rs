use std::marker::PhantomData;

use crate::drizzle_builder_join_impl;

use drizzle_core::ConflictTarget;
use drizzle_core::traits::{SQLModel, SQLTable, ToSQL};
use drizzle_sqlite::{
    builder::{
        self, CTEView, DeleteInitial, DeleteWhereSet, InsertDoUpdateSet, InsertInitial,
        InsertOnConflictSet, InsertReturningSet, InsertValuesSet, OnConflictBuilder, QueryBuilder,
        SelectFromSet, SelectInitial, SelectJoinSet, SelectLimitSet, SelectOffsetSet,
        SelectOrderSet, SelectWhereSet, UpdateInitial, UpdateSetClauseSet, UpdateWhereSet,
        delete::DeleteBuilder,
        insert::InsertBuilder,
        select::{AsCteState, SelectBuilder},
        update::UpdateBuilder,
    },
    common::SQLiteSchemaType,
    traits::SQLiteTable,
    values::SQLiteValue,
};

/// Shared SQLite drizzle builder wrapper for all SQLite drivers.
#[derive(Debug)]
pub struct DrizzleBuilder<'a, Conn, Schema, Builder, State> {
    pub(crate) drizzle: &'a Drizzle<Conn, Schema>,
    pub(crate) builder: Builder,
    pub(crate) state: PhantomData<(Schema, State)>,
}

/// Intermediate builder for typed ON CONFLICT within a Drizzle wrapper.
pub struct DrizzleOnConflictBuilder<'a, 'b, Conn, Schema, Table> {
    drizzle: &'a Drizzle<Conn, Schema>,
    builder: OnConflictBuilder<'b, Schema, Table>,
}

impl<'a, 'b, Conn, Schema, Table> DrizzleOnConflictBuilder<'a, 'b, Conn, Schema, Table> {
    /// Adds a WHERE clause to the conflict target for partial index matching.
    pub fn r#where(mut self, condition: impl ToSQL<'b, SQLiteValue<'b>>) -> Self {
        self.builder = self.builder.r#where(condition);
        self
    }

    /// `ON CONFLICT (cols) DO NOTHING`
    pub fn do_nothing(
        self,
    ) -> DrizzleBuilder<
        'a,
        Conn,
        Schema,
        InsertBuilder<'b, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    > {
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder: self.builder.do_nothing(),
            state: PhantomData,
        }
    }

    /// `ON CONFLICT (cols) DO UPDATE SET ...`
    pub fn do_update(
        self,
        set: impl ToSQL<'b, SQLiteValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        Conn,
        Schema,
        InsertBuilder<'b, Schema, InsertDoUpdateSet, Table>,
        InsertDoUpdateSet,
    > {
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder: self.builder.do_update(set),
            state: PhantomData,
        }
    }
}

/// Shared SQLite drizzle connection wrapper.
#[derive(Debug)]
pub struct Drizzle<Conn, Schema = ()> {
    pub(crate) conn: Conn,
    pub(crate) _schema: PhantomData<Schema>,
}

impl<Conn> Drizzle<Conn> {
    /// Creates a new `Drizzle` instance.
    ///
    /// Returns a tuple of (Drizzle, Schema) for destructuring.
    #[inline]
    pub const fn new<S>(conn: Conn, schema: S) -> (Drizzle<Conn, S>, S) {
        let drizzle = Drizzle {
            conn,
            _schema: PhantomData,
        };
        (drizzle, schema)
    }
}

impl<Conn, S> AsRef<Drizzle<Conn, S>> for Drizzle<Conn, S> {
    #[inline]
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<Conn, Schema> Drizzle<Conn, Schema> {
    /// Gets a reference to the underlying connection.
    #[inline]
    pub fn conn(&self) -> &Conn {
        &self.conn
    }

    /// Gets a mutable reference to the underlying connection.
    #[inline]
    pub fn mut_conn(&mut self) -> &mut Conn {
        &mut self.conn
    }

    /// Creates a SELECT query builder.
    #[cfg(feature = "sqlite")]
    pub fn select<'a, 'b, T>(
        &'a self,
        query: T,
    ) -> DrizzleBuilder<
        'a,
        Conn,
        Schema,
        SelectBuilder<'b, Schema, drizzle_sqlite::builder::select::SelectInitial>,
        drizzle_sqlite::builder::select::SelectInitial,
    >
    where
        T: ToSQL<'b, SQLiteValue<'b>>,
    {
        let builder = QueryBuilder::new::<Schema>().select(query);

        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates a SELECT DISTINCT query builder.
    #[cfg(feature = "sqlite")]
    pub fn select_distinct<'a, 'b, T>(
        &'a self,
        query: T,
    ) -> DrizzleBuilder<
        'a,
        Conn,
        Schema,
        SelectBuilder<'b, Schema, drizzle_sqlite::builder::select::SelectInitial>,
        drizzle_sqlite::builder::select::SelectInitial,
    >
    where
        T: ToSQL<'b, SQLiteValue<'b>>,
    {
        let builder = QueryBuilder::new::<Schema>().select_distinct(query);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates an INSERT query builder.
    #[cfg(feature = "sqlite")]
    pub fn insert<'a, 'b, Table>(
        &'a self,
        table: Table,
    ) -> DrizzleBuilder<
        'a,
        Conn,
        Schema,
        InsertBuilder<'b, Schema, drizzle_sqlite::builder::insert::InsertInitial, Table>,
        drizzle_sqlite::builder::insert::InsertInitial,
    >
    where
        Table: SQLiteTable<'b>,
    {
        let builder = QueryBuilder::new::<Schema>().insert(table);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates an UPDATE query builder.
    #[cfg(feature = "sqlite")]
    pub fn update<'a, 'b, Table>(
        &'a self,
        table: Table,
    ) -> DrizzleBuilder<
        'a,
        Conn,
        Schema,
        UpdateBuilder<'b, Schema, drizzle_sqlite::builder::update::UpdateInitial, Table>,
        drizzle_sqlite::builder::update::UpdateInitial,
    >
    where
        Table: SQLiteTable<'b>,
    {
        let builder = QueryBuilder::new::<Schema>().update(table);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates a DELETE query builder.
    #[cfg(feature = "sqlite")]
    pub fn delete<'a, 'b, Table>(
        &'a self,
        table: Table,
    ) -> DrizzleBuilder<
        'a,
        Conn,
        Schema,
        DeleteBuilder<'b, Schema, drizzle_sqlite::builder::delete::DeleteInitial, Table>,
        drizzle_sqlite::builder::delete::DeleteInitial,
    >
    where
        Table: SQLiteTable<'b>,
    {
        let builder = QueryBuilder::new::<Schema>().delete(table);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates a query with CTE (Common Table Expression).
    #[cfg(feature = "sqlite")]
    pub fn with<'a, 'b, C>(
        &'a self,
        cte: C,
    ) -> DrizzleBuilder<
        'a,
        Conn,
        Schema,
        QueryBuilder<'b, Schema, builder::CTEInit>,
        builder::CTEInit,
    >
    where
        C: builder::CTEDefinition<'b>,
    {
        let builder = QueryBuilder::new::<Schema>().with(cte);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }
}

impl<'d, 'a, Conn, S, T, State> ToSQL<'a, SQLiteValue<'a>> for DrizzleBuilder<'d, Conn, S, T, State>
where
    T: ToSQL<'a, SQLiteValue<'a>>,
{
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, SQLiteValue<'a>> {
        self.builder.to_sql()
    }
}

impl<'d, 'a, Conn, S, T, State> drizzle_core::expr::Expr<'a, SQLiteValue<'a>>
    for DrizzleBuilder<'d, Conn, S, T, State>
where
    T: ToSQL<'a, SQLiteValue<'a>>,
{
    type SQLType = drizzle_core::types::Any;
    type Nullable = drizzle_core::expr::NonNull;
    type Aggregate = drizzle_core::expr::Scalar;
}

impl<'d, 'a, Conn, Schema>
    DrizzleBuilder<'d, Conn, Schema, QueryBuilder<'a, Schema, builder::CTEInit>, builder::CTEInit>
{
    #[inline]
    pub fn select<T>(
        self,
        query: T,
    ) -> DrizzleBuilder<
        'd,
        Conn,
        Schema,
        SelectBuilder<'a, Schema, builder::select::SelectInitial>,
        builder::select::SelectInitial,
    >
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.select(query);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    #[inline]
    pub fn select_distinct<T>(
        self,
        query: T,
    ) -> DrizzleBuilder<
        'd,
        Conn,
        Schema,
        SelectBuilder<'a, Schema, builder::select::SelectInitial>,
        builder::select::SelectInitial,
    >
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.select_distinct(query);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    #[inline]
    pub fn with<C>(
        self,
        cte: C,
    ) -> DrizzleBuilder<
        'd,
        Conn,
        Schema,
        QueryBuilder<'a, Schema, builder::CTEInit>,
        builder::CTEInit,
    >
    where
        C: builder::CTEDefinition<'a>,
    {
        let builder = self.builder.with(cte);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'d, 'a, Conn, Schema>
    DrizzleBuilder<'d, Conn, Schema, SelectBuilder<'a, Schema, SelectInitial>, SelectInitial>
{
    #[inline]
    pub fn from<T>(
        self,
        table: T,
    ) -> DrizzleBuilder<'d, Conn, Schema, SelectBuilder<'a, Schema, SelectFromSet, T>, SelectFromSet>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.from(table);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'d, 'a, Conn, Schema, T>
    DrizzleBuilder<'d, Conn, Schema, SelectBuilder<'a, Schema, SelectFromSet, T>, SelectFromSet>
{
    #[inline]
    pub fn r#where(
        self,
        condition: impl drizzle_core::traits::ToSQL<'a, SQLiteValue<'a>>,
    ) -> DrizzleBuilder<
        'd,
        Conn,
        Schema,
        SelectBuilder<'a, Schema, SelectWhereSet, T>,
        SelectWhereSet,
    > {
        let builder = self.builder.r#where(condition.to_sql());
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    #[inline]
    pub fn limit(
        self,
        limit: usize,
    ) -> DrizzleBuilder<
        'd,
        Conn,
        Schema,
        SelectBuilder<'a, Schema, SelectLimitSet, T>,
        SelectLimitSet,
    > {
        let builder = self.builder.limit(limit);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> DrizzleBuilder<
        'd,
        Conn,
        Schema,
        SelectBuilder<'a, Schema, SelectOrderSet, T>,
        SelectOrderSet,
    >
    where
        TOrderBy: drizzle_core::traits::ToSQL<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.order_by(expressions);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    #[inline]
    pub fn join<J: drizzle_sqlite::helpers::JoinArg<'a, T>>(
        self,
        arg: J,
    ) -> DrizzleBuilder<
        'd,
        Conn,
        Schema,
        SelectBuilder<'a, Schema, SelectJoinSet, J::JoinedTable>,
        SelectJoinSet,
    > {
        let builder = self.builder.join(arg);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    crate::drizzle_builder_join_impl!();
}

impl<'d, 'a, Conn, Schema, T>
    DrizzleBuilder<'d, Conn, Schema, SelectBuilder<'a, Schema, SelectJoinSet, T>, SelectJoinSet>
{
    pub fn r#where(
        self,
        condition: impl drizzle_core::traits::ToSQL<'a, SQLiteValue<'a>>,
    ) -> DrizzleBuilder<
        'd,
        Conn,
        Schema,
        SelectBuilder<'a, Schema, SelectWhereSet, T>,
        SelectWhereSet,
    > {
        let builder = self.builder.r#where(condition.to_sql());
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> DrizzleBuilder<
        'd,
        Conn,
        Schema,
        SelectBuilder<'a, Schema, SelectOrderSet, T>,
        SelectOrderSet,
    >
    where
        TOrderBy: drizzle_core::traits::ToSQL<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.order_by(expressions);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    pub fn join<J: drizzle_sqlite::helpers::JoinArg<'a, T>>(
        self,
        arg: J,
    ) -> DrizzleBuilder<
        'd,
        Conn,
        Schema,
        SelectBuilder<'a, Schema, SelectJoinSet, J::JoinedTable>,
        SelectJoinSet,
    > {
        let builder = self.builder.join(arg);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    crate::drizzle_builder_join_impl!();
}

impl<'d, 'a, Conn, Schema, T>
    DrizzleBuilder<'d, Conn, Schema, SelectBuilder<'a, Schema, SelectWhereSet, T>, SelectWhereSet>
{
    pub fn limit(
        self,
        limit: usize,
    ) -> DrizzleBuilder<
        'd,
        Conn,
        Schema,
        SelectBuilder<'a, Schema, SelectLimitSet, T>,
        SelectLimitSet,
    > {
        let builder = self.builder.limit(limit);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> DrizzleBuilder<
        'd,
        Conn,
        Schema,
        SelectBuilder<'a, Schema, SelectOrderSet, T>,
        SelectOrderSet,
    >
    where
        TOrderBy: drizzle_core::traits::ToSQL<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.order_by(expressions);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'d, 'a, Conn, Schema, T>
    DrizzleBuilder<'d, Conn, Schema, SelectBuilder<'a, Schema, SelectLimitSet, T>, SelectLimitSet>
{
    pub fn offset(
        self,
        offset: usize,
    ) -> DrizzleBuilder<
        'd,
        Conn,
        Schema,
        SelectBuilder<'a, Schema, SelectOffsetSet, T>,
        SelectOffsetSet,
    > {
        let builder = self.builder.offset(offset);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'d, 'a, Conn, Schema, T>
    DrizzleBuilder<'d, Conn, Schema, SelectBuilder<'a, Schema, SelectOrderSet, T>, SelectOrderSet>
{
    pub fn limit(
        self,
        limit: usize,
    ) -> DrizzleBuilder<
        'd,
        Conn,
        Schema,
        SelectBuilder<'a, Schema, SelectLimitSet, T>,
        SelectLimitSet,
    > {
        let builder = self.builder.limit(limit);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'d, 'a, Conn, Schema, State, T>
    DrizzleBuilder<'d, Conn, Schema, SelectBuilder<'a, Schema, State, T>, State>
where
    State: AsCteState,
    T: SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>>,
{
    /// Converts this SELECT query into a CTE (Common Table Expression) with the given name.
    #[inline]
    pub fn into_cte(
        self,
        name: &'static str,
    ) -> CTEView<
        'a,
        <T as SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>>>::Aliased,
        SelectBuilder<'a, Schema, State, T>,
    > {
        self.builder.into_cte(name)
    }
}

impl<'a, 'b, Conn, Schema, Table>
    DrizzleBuilder<'a, Conn, Schema, InsertBuilder<'b, Schema, InsertInitial, Table>, InsertInitial>
{
    #[inline]
    pub fn values<T>(
        self,
        values: impl IntoIterator<Item = Table::Insert<T>>,
    ) -> DrizzleBuilder<
        'a,
        Conn,
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

impl<'a, 'b, Conn, Schema, Table>
    DrizzleBuilder<
        'a,
        Conn,
        Schema,
        InsertBuilder<'b, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
where
    Table: SQLiteTable<'b>,
{
    /// Begins a typed ON CONFLICT clause targeting a specific constraint.
    pub fn on_conflict<C: ConflictTarget<Table>>(
        self,
        target: C,
    ) -> DrizzleOnConflictBuilder<'a, 'b, Conn, Schema, Table> {
        DrizzleOnConflictBuilder {
            drizzle: self.drizzle,
            builder: self.builder.on_conflict(target),
        }
    }

    /// Shorthand for `ON CONFLICT DO NOTHING` without specifying a target.
    pub fn on_conflict_do_nothing(
        self,
    ) -> DrizzleBuilder<
        'a,
        Conn,
        Schema,
        InsertBuilder<'b, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    > {
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder: self.builder.on_conflict_do_nothing(),
            state: PhantomData,
        }
    }

    /// Adds RETURNING clause
    pub fn returning(
        self,
        columns: impl ToSQL<'b, SQLiteValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        Conn,
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

impl<'a, 'b, Conn, Schema, Table>
    DrizzleBuilder<
        'a,
        Conn,
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
        Conn,
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

impl<'a, 'b, Conn, Schema, Table>
    DrizzleBuilder<
        'a,
        Conn,
        Schema,
        InsertBuilder<'b, Schema, InsertDoUpdateSet, Table>,
        InsertDoUpdateSet,
    >
{
    /// Adds WHERE clause after DO UPDATE SET
    pub fn r#where(
        self,
        condition: impl ToSQL<'b, SQLiteValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        Conn,
        Schema,
        InsertBuilder<'b, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    > {
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder: self.builder.r#where(condition),
            state: PhantomData,
        }
    }

    /// Adds RETURNING clause after DO UPDATE SET
    pub fn returning(
        self,
        columns: impl ToSQL<'b, SQLiteValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        Conn,
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

impl<'a, 'b, Conn, Schema, Table>
    DrizzleBuilder<'a, Conn, Schema, UpdateBuilder<'b, Schema, UpdateInitial, Table>, UpdateInitial>
where
    Table: SQLiteTable<'b>,
{
    #[inline]
    pub fn set(
        self,
        values: Table::Update,
    ) -> DrizzleBuilder<
        'a,
        Conn,
        Schema,
        UpdateBuilder<'b, Schema, UpdateSetClauseSet, Table>,
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

impl<'a, 'b, Conn, Schema, Table>
    DrizzleBuilder<
        'a,
        Conn,
        Schema,
        UpdateBuilder<'b, Schema, UpdateSetClauseSet, Table>,
        UpdateSetClauseSet,
    >
{
    pub fn r#where(
        self,
        condition: impl drizzle_core::traits::ToSQL<'b, SQLiteValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        Conn,
        Schema,
        UpdateBuilder<'b, Schema, UpdateWhereSet, Table>,
        UpdateWhereSet,
    > {
        let builder = self.builder.r#where(condition.to_sql());
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Conn, Schema, T>
    DrizzleBuilder<'a, Conn, Schema, DeleteBuilder<'b, Schema, DeleteInitial, T>, DeleteInitial>
where
    T: SQLiteTable<'b>,
{
    pub fn r#where(
        self,
        condition: impl drizzle_core::traits::ToSQL<'b, SQLiteValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        Conn,
        Schema,
        DeleteBuilder<'b, Schema, DeleteWhereSet, T>,
        DeleteWhereSet,
    > {
        let builder = self.builder.r#where(condition.to_sql());
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}
