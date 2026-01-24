use std::marker::PhantomData;

use crate::drizzle_builder_join_impl;

use drizzle_core::traits::{SQLModel, SQLTable, ToSQL};
use drizzle_sqlite::{
    builder::{
        self, CTEView, Conflict, DeleteInitial, DeleteWhereSet, InsertInitial, InsertOnConflictSet,
        InsertReturningSet, InsertValuesSet, QueryBuilder, SelectFromSet, SelectInitial,
        SelectJoinSet, SelectLimitSet, SelectOffsetSet, SelectOrderSet, SelectWhereSet,
        UpdateInitial, UpdateSetClauseSet, UpdateWhereSet, delete::DeleteBuilder,
        insert::InsertBuilder, select::SelectBuilder, update::UpdateBuilder,
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

// CTE (WITH) Builder Implementation
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

//------------------------------------------------------------------------------
// SELECT builder wrappers
//------------------------------------------------------------------------------

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
    pub fn join<U>(
        self,
        table: U,
        on_condition: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> DrizzleBuilder<'d, Conn, Schema, SelectBuilder<'a, Schema, SelectJoinSet, T>, SelectJoinSet>
    where
        U: SQLiteTable<'a>,
    {
        let builder = self.builder.join(table, on_condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    /// Converts this SELECT query into a CTE (Common Table Expression) with the given name.
    /// Returns a CTEView with typed field access via Deref to the aliased table.
    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub fn as_cte(
        self,
        name: &'static str,
    ) -> CTEView<
        'a,
        <T as SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>>>::Aliased,
        SelectBuilder<'a, Schema, SelectFromSet, T>,
    >
    where
        T: SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>>,
    {
        self.builder.as_cte(name)
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

    pub fn join<U>(
        self,
        table: U,
        condition: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> DrizzleBuilder<'d, Conn, Schema, SelectBuilder<'a, Schema, SelectJoinSet, T>, SelectJoinSet>
    where
        U: SQLiteTable<'a>,
    {
        let builder = self.builder.join(table, condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    /// Converts this SELECT query into a CTE (Common Table Expression) with the given name.
    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub fn as_cte(
        self,
        name: &'static str,
    ) -> CTEView<
        'a,
        <T as SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>>>::Aliased,
        SelectBuilder<'a, Schema, SelectJoinSet, T>,
    >
    where
        T: SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>>,
    {
        self.builder.as_cte(name)
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

    /// Converts this SELECT query into a CTE (Common Table Expression) with the given name.
    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub fn as_cte(
        self,
        name: &'static str,
    ) -> CTEView<
        'a,
        <T as SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>>>::Aliased,
        SelectBuilder<'a, Schema, SelectWhereSet, T>,
    >
    where
        T: SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>>,
    {
        self.builder.as_cte(name)
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

    /// Converts this SELECT query into a CTE (Common Table Expression) with the given name.
    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub fn as_cte(
        self,
        name: &'static str,
    ) -> CTEView<
        'a,
        <T as SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>>>::Aliased,
        SelectBuilder<'a, Schema, SelectLimitSet, T>,
    >
    where
        T: SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>>,
    {
        self.builder.as_cte(name)
    }
}

impl<'d, 'a, Conn, Schema, T>
    DrizzleBuilder<'d, Conn, Schema, SelectBuilder<'a, Schema, SelectOffsetSet, T>, SelectOffsetSet>
{
    /// Converts this SELECT query into a CTE (Common Table Expression) with the given name.
    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub fn as_cte(
        self,
        name: &'static str,
    ) -> CTEView<
        'a,
        <T as SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>>>::Aliased,
        SelectBuilder<'a, Schema, SelectOffsetSet, T>,
    >
    where
        T: SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>>,
    {
        self.builder.as_cte(name)
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

    /// Converts this SELECT query into a CTE (Common Table Expression) with the given name.
    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub fn as_cte(
        self,
        name: &'static str,
    ) -> CTEView<
        'a,
        <T as SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>>>::Aliased,
        SelectBuilder<'a, Schema, SelectOrderSet, T>,
    >
    where
        T: SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>>,
    {
        self.builder.as_cte(name)
    }
}

//------------------------------------------------------------------------------
// INSERT builder wrappers
//------------------------------------------------------------------------------

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
    /// Adds conflict resolution clause
    pub fn on_conflict<TI>(
        self,
        conflict: Conflict<'b, TI>,
    ) -> DrizzleBuilder<
        'a,
        Conn,
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

//------------------------------------------------------------------------------
// UPDATE builder wrappers
//------------------------------------------------------------------------------

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

//------------------------------------------------------------------------------
// DELETE builder wrappers
//------------------------------------------------------------------------------

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
