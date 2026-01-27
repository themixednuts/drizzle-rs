use std::marker::PhantomData;

use drizzle_core::traits::{SQLModel, SQLTable, ToSQL};
use drizzle_postgres::builder::{
    self, CTEView, Conflict, DeleteInitial, DeleteReturningSet, DeleteWhereSet, InsertInitial,
    InsertOnConflictSet, InsertReturningSet, InsertValuesSet, QueryBuilder, SelectFromSet,
    SelectInitial, SelectJoinSet, SelectLimitSet, SelectOffsetSet, SelectOrderSet, SelectWhereSet,
    UpdateFromSet, UpdateInitial, UpdateReturningSet, UpdateSetClauseSet, UpdateWhereSet,
    delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder, update::UpdateBuilder,
};
use drizzle_postgres::common::PostgresSchemaType;
use drizzle_postgres::traits::PostgresTable;
use drizzle_postgres::values::PostgresValue;

/// Shared Postgres drizzle builder wrapper.
#[derive(Debug)]
pub struct DrizzleBuilder<'a, DrizzleRef, Schema, Builder, State> {
    pub(crate) drizzle: DrizzleRef,
    pub(crate) builder: Builder,
    pub(crate) state: PhantomData<(Schema, State, &'a ())>,
}

impl<'d, 'a, DrizzleRef, S, T, State> ToSQL<'a, PostgresValue<'a>>
    for DrizzleBuilder<'d, DrizzleRef, S, T, State>
where
    T: ToSQL<'a, PostgresValue<'a>>,
{
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, PostgresValue<'a>> {
        self.builder.to_sql()
    }
}

impl<'d, 'a, DrizzleRef, S, T, State> drizzle_core::expr::Expr<'a, PostgresValue<'a>>
    for DrizzleBuilder<'d, DrizzleRef, S, T, State>
where
    T: ToSQL<'a, PostgresValue<'a>>,
{
    type SQLType = drizzle_core::types::Any;
    type Nullable = drizzle_core::expr::NonNull;
    type Aggregate = drizzle_core::expr::Scalar;
}

impl<'d, 'a, DrizzleRef, Schema>
    DrizzleBuilder<
        'd,
        DrizzleRef,
        Schema,
        QueryBuilder<'a, Schema, builder::CTEInit>,
        builder::CTEInit,
    >
{
    #[inline]
    pub fn select<T>(
        self,
        query: T,
    ) -> DrizzleBuilder<
        'd,
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, builder::select::SelectInitial>,
        builder::select::SelectInitial,
    >
    where
        T: ToSQL<'a, PostgresValue<'a>>,
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
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, builder::select::SelectInitial>,
        builder::select::SelectInitial,
    >
    where
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let builder = self.builder.select_distinct(query);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    #[inline]
    pub fn select_distinct_on<On, Columns>(
        self,
        on: On,
        columns: Columns,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, builder::select::SelectInitial>,
        builder::select::SelectInitial,
    >
    where
        On: ToSQL<'a, PostgresValue<'a>>,
        Columns: ToSQL<'a, PostgresValue<'a>>,
    {
        let builder = self.builder.select_distinct_on(on, columns);
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
        DrizzleRef,
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

impl<'d, 'a, DrizzleRef, Schema>
    DrizzleBuilder<'d, DrizzleRef, Schema, SelectBuilder<'a, Schema, SelectInitial>, SelectInitial>
{
    #[inline]
    pub fn from<T>(
        self,
        table: T,
    ) -> DrizzleBuilder<
        'd,
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, SelectFromSet, T>,
        SelectFromSet,
    >
    where
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        let builder = self.builder.from(table);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'d, 'a, DrizzleRef, Schema, T>
    DrizzleBuilder<
        'd,
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, SelectFromSet, T>,
        SelectFromSet,
    >
{
    #[inline]
    pub fn r#where(
        self,
        condition: impl drizzle_core::traits::ToSQL<'a, PostgresValue<'a>>,
    ) -> DrizzleBuilder<
        'd,
        DrizzleRef,
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
        DrizzleRef,
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
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, SelectOrderSet, T>,
        SelectOrderSet,
    >
    where
        TOrderBy: drizzle_core::traits::ToSQL<'a, PostgresValue<'a>>,
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
        on_condition: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> DrizzleBuilder<
        'd,
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, SelectJoinSet, T>,
        SelectJoinSet,
    >
    where
        U: PostgresTable<'a>,
    {
        let builder = self.builder.join(table, on_condition.to_sql());
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
        <T as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Aliased,
        SelectBuilder<'a, Schema, SelectFromSet, T>,
    >
    where
        T: SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>,
    {
        self.builder.as_cte(name)
    }
}

impl<'d, 'a, DrizzleRef, Schema, T>
    DrizzleBuilder<
        'd,
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, SelectJoinSet, T>,
        SelectJoinSet,
    >
{
    pub fn r#where(
        self,
        condition: impl drizzle_core::traits::ToSQL<'a, PostgresValue<'a>>,
    ) -> DrizzleBuilder<
        'd,
        DrizzleRef,
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
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, SelectOrderSet, T>,
        SelectOrderSet,
    >
    where
        TOrderBy: drizzle_core::traits::ToSQL<'a, PostgresValue<'a>>,
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
        condition: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> DrizzleBuilder<
        'd,
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, SelectJoinSet, T>,
        SelectJoinSet,
    >
    where
        U: PostgresTable<'a>,
    {
        let builder = self.builder.join(table, condition.to_sql());
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
        <T as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Aliased,
        SelectBuilder<'a, Schema, SelectJoinSet, T>,
    >
    where
        T: SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>,
    {
        self.builder.as_cte(name)
    }
}

impl<'d, 'a, DrizzleRef, Schema, T>
    DrizzleBuilder<
        'd,
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, SelectWhereSet, T>,
        SelectWhereSet,
    >
{
    pub fn limit(
        self,
        limit: usize,
    ) -> DrizzleBuilder<
        'd,
        DrizzleRef,
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
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, SelectOrderSet, T>,
        SelectOrderSet,
    >
    where
        TOrderBy: drizzle_core::traits::ToSQL<'a, PostgresValue<'a>>,
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
        <T as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Aliased,
        SelectBuilder<'a, Schema, SelectWhereSet, T>,
    >
    where
        T: SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>,
    {
        self.builder.as_cte(name)
    }
}

impl<'d, 'a, DrizzleRef, Schema, T>
    DrizzleBuilder<
        'd,
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, SelectLimitSet, T>,
        SelectLimitSet,
    >
{
    pub fn offset(
        self,
        offset: usize,
    ) -> DrizzleBuilder<
        'd,
        DrizzleRef,
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
        <T as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Aliased,
        SelectBuilder<'a, Schema, SelectLimitSet, T>,
    >
    where
        T: SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>,
    {
        self.builder.as_cte(name)
    }
}

impl<'d, 'a, DrizzleRef, Schema, T>
    DrizzleBuilder<
        'd,
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, SelectOffsetSet, T>,
        SelectOffsetSet,
    >
{
    /// Converts this SELECT query into a CTE (Common Table Expression) with the given name.
    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub fn as_cte(
        self,
        name: &'static str,
    ) -> CTEView<
        'a,
        <T as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Aliased,
        SelectBuilder<'a, Schema, SelectOffsetSet, T>,
    >
    where
        T: SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>,
    {
        self.builder.as_cte(name)
    }
}

impl<'d, 'a, DrizzleRef, Schema, T>
    DrizzleBuilder<
        'd,
        DrizzleRef,
        Schema,
        SelectBuilder<'a, Schema, SelectOrderSet, T>,
        SelectOrderSet,
    >
{
    pub fn limit(
        self,
        limit: usize,
    ) -> DrizzleBuilder<
        'd,
        DrizzleRef,
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
        <T as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Aliased,
        SelectBuilder<'a, Schema, SelectOrderSet, T>,
    >
    where
        T: SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>,
    {
        self.builder.as_cte(name)
    }
}

impl<'a, 'b, DrizzleRef, Schema, Table>
    DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        InsertBuilder<'b, Schema, InsertInitial, Table>,
        InsertInitial,
    >
{
    #[inline]
    pub fn values<T>(
        self,
        values: impl IntoIterator<Item = Table::Insert<T>>,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        InsertBuilder<'b, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
    where
        Table: PostgresTable<'b>,
        Table::Insert<T>: SQLModel<'b, PostgresValue<'b>>,
    {
        let builder = self.builder.values(values);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, DrizzleRef, Schema, Table>
    DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        InsertBuilder<'b, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
where
    Table: PostgresTable<'b>,
{
    /// Adds conflict resolution clause
    pub fn on_conflict(
        self,
        conflict: Conflict<'b>,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        InsertBuilder<'b, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    > {
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
        columns: impl ToSQL<'b, PostgresValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
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

impl<'a, 'b, DrizzleRef, Schema, Table>
    DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        InsertBuilder<'b, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    >
{
    /// Adds RETURNING clause after ON CONFLICT
    pub fn returning(
        self,
        columns: impl ToSQL<'b, PostgresValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
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

impl<'a, 'b, DrizzleRef, Schema, Table>
    DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        UpdateBuilder<'b, Schema, UpdateInitial, Table>,
        UpdateInitial,
    >
where
    Table: PostgresTable<'b>,
{
    #[inline]
    pub fn set(
        self,
        values: Table::Update,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
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

impl<'a, 'b, DrizzleRef, Schema, Table>
    DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        UpdateBuilder<'b, Schema, UpdateSetClauseSet, Table>,
        UpdateSetClauseSet,
    >
{
    pub fn from(
        self,
        source: impl ToSQL<'b, PostgresValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        UpdateBuilder<'b, Schema, UpdateFromSet, Table>,
        UpdateFromSet,
    > {
        let builder = self.builder.from(source.to_sql());
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    pub fn r#where(
        self,
        condition: impl ToSQL<'b, PostgresValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
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

    pub fn returning(
        self,
        columns: impl ToSQL<'b, PostgresValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        UpdateBuilder<'b, Schema, UpdateReturningSet, Table>,
        UpdateReturningSet,
    > {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, DrizzleRef, Schema, Table>
    DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        UpdateBuilder<'b, Schema, UpdateFromSet, Table>,
        UpdateFromSet,
    >
{
    pub fn r#where(
        self,
        condition: impl ToSQL<'b, PostgresValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
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

    pub fn returning(
        self,
        columns: impl ToSQL<'b, PostgresValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        UpdateBuilder<'b, Schema, UpdateReturningSet, Table>,
        UpdateReturningSet,
    > {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, DrizzleRef, Schema, Table>
    DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        UpdateBuilder<'b, Schema, UpdateWhereSet, Table>,
        UpdateWhereSet,
    >
{
    pub fn returning(
        self,
        columns: impl ToSQL<'b, PostgresValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        UpdateBuilder<'b, Schema, UpdateReturningSet, Table>,
        UpdateReturningSet,
    > {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, DrizzleRef, Schema, Table>
    DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        DeleteBuilder<'b, Schema, DeleteInitial, Table>,
        DeleteInitial,
    >
where
    Table: PostgresTable<'b>,
{
    pub fn r#where(
        self,
        condition: impl ToSQL<'b, PostgresValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        DeleteBuilder<'b, Schema, DeleteWhereSet, Table>,
        DeleteWhereSet,
    > {
        let builder = self.builder.r#where(condition.to_sql());
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    pub fn returning(
        self,
        columns: impl ToSQL<'b, PostgresValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        DeleteBuilder<'b, Schema, DeleteReturningSet, Table>,
        DeleteReturningSet,
    > {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, DrizzleRef, Schema, Table>
    DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        DeleteBuilder<'b, Schema, DeleteWhereSet, Table>,
        DeleteWhereSet,
    >
{
    pub fn returning(
        self,
        columns: impl ToSQL<'b, PostgresValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        DrizzleRef,
        Schema,
        DeleteBuilder<'b, Schema, DeleteReturningSet, Table>,
        DeleteReturningSet,
    > {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}
