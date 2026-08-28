//! Type-safe MySQL query construction.

use crate::{common::MySQLSchemaType, traits::MySQLTable, values::MySQLValue};
use core::{fmt::Debug, marker::PhantomData};
use drizzle_core::{SQL, ToSQL, Token};

pub use drizzle_core::{BuilderInit, ExecutableState};

macro_rules! mutation_builder_methods {
    (
        $builder:ident,
        prepare: [$($prepare:ty),+ $(,)?],
        order_by: [$($order:ty),+ $(,)?] => $ordered:ty,
        limit: [$($limit:ty),+ $(,)?] => $limited:ty $(,)?
    ) => {
        $(
            impl<'a, S, T, M, R> $builder<'a, S, $prepare, T, M, R> {
                /// Compiles this mutation into a reusable prepared statement.
                #[must_use]
                pub fn prepare(
                    &self,
                ) -> drizzle_core::prepared::PreparedStatement<'a, crate::values::MySQLValue<'a>> {
                    self.prepared_statement()
                }
            }
        )+

        $(
            impl<'a, S, T> $builder<'a, S, $order, T> {
                /// Orders the rows considered by this mutation.
                pub fn order_by<O>(self, order: O) -> $builder<'a, S, $ordered, T>
                where
                    O: drizzle_core::ToSQL<'a, crate::values::MySQLValue<'a>>,
                {
                    $builder::from_sql(self.sql.append(crate::helpers::order_by(order)))
                }
            }
        )+

        $(
            impl<'a, S, T> $builder<'a, S, $limit, T> {
                /// Limits the number of rows affected by this mutation.
                #[track_caller]
                pub fn limit<P>(self, limit: P) -> $builder<'a, S, $limited, T>
                where
                    P: drizzle_core::PaginationArg<'a, crate::values::MySQLValue<'a>>,
                {
                    $builder::from_sql(self.sql.append(crate::helpers::limit(limit)))
                }
            }
        )+
    };
}

pub mod cte;
/// Typed MySQL `DELETE` statements.
pub mod delete;
/// Typed MySQL `INSERT` statements.
pub mod insert;
/// Typed MySQL `SELECT` statements.
pub mod select;
/// Typed MySQL `UPDATE` statements.
pub mod update;

pub use cte::{CTEDefinition, CTEView};
pub use delete::{DeleteBuilder, DeleteInitial, DeleteLimitSet, DeleteOrderSet, DeleteWhereSet};
pub use insert::{
    InsertBuilder, InsertColumnsSet, InsertIgnoreSet, InsertInitial, InsertOnDuplicateKeyUpdateSet,
    InsertValuesSet,
};
pub use select::{
    CompletedSelect, ForShare, ForUpdate, IntoSelectQuery, NoWait, SelectBuilder, SelectForSet,
    SelectFromSet, SelectGroupSet, SelectHavingSet, SelectIndexHintSet, SelectInitial,
    SelectJoinSet, SelectLimitSet, SelectOffsetSet, SelectOrderSet, SelectSetOpSet, SelectWhereSet,
    SkipLocked, Wait,
};
pub use update::{
    UpdateBuilder, UpdateInitial, UpdateLimitSet, UpdateOrderSet, UpdateSetClauseSet,
    UpdateWhereSet,
};

/// Builder state after attaching a common table expression.
#[derive(Debug, Clone)]
pub struct CTEInit;

/// Driver-neutral entry point for MySQL SQL construction.
#[derive(Debug, Clone)]
pub struct QueryBuilder<
    'a,
    Schema = (),
    State = (),
    Table = (),
    Marker = (),
    Row = (),
    Grouped = (),
> {
    pub(crate) sql: SQL<'a, MySQLValue<'a>>,
    pub(crate) schema: PhantomData<Schema>,
    pub(crate) state: PhantomData<State>,
    pub(crate) table: PhantomData<Table>,
    pub(crate) marker: PhantomData<Marker>,
    pub(crate) row: PhantomData<Row>,
    pub(crate) grouped: PhantomData<Grouped>,
}

impl<'a, Schema, State, Table, Marker, Row, Grouped> ToSQL<'a, MySQLValue<'a>>
    for QueryBuilder<'a, Schema, State, Table, Marker, Row, Grouped>
where
    State: ExecutableState,
{
    fn to_sql(&self) -> SQL<'a, MySQLValue<'a>> {
        self.sql.clone()
    }

    fn into_sql(self) -> SQL<'a, MySQLValue<'a>> {
        self.sql
    }
}

impl<'a, Schema, State, Table, Marker, Row, Grouped>
    QueryBuilder<'a, Schema, State, Table, Marker, Row, Grouped>
where
    State: ExecutableState,
{
    /// Prepends a sanitized SQL comment to this query.
    #[must_use]
    pub fn comment(mut self, text: impl AsRef<str>) -> Self {
        let fragment = drizzle_core::sql::comment::<MySQLValue<'a>>(text);
        if !fragment.chunks.is_empty() {
            let existing = core::mem::replace(&mut self.sql, fragment);
            self.sql.append_mut(existing);
        }
        self
    }

    /// Prepends structured, sanitized key-value tags as a SQL comment.
    #[must_use]
    pub fn comment_tags<I, K, V>(mut self, pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let fragment = drizzle_core::sql::comment_tags::<MySQLValue<'a>, _, _, _>(pairs);
        if !fragment.chunks.is_empty() {
            let existing = core::mem::replace(&mut self.sql, fragment);
            self.sql.append_mut(existing);
        }
        self
    }
}

impl<'a> QueryBuilder<'a> {
    /// Creates an empty builder for schema `S`.
    #[must_use]
    pub const fn new<S>() -> QueryBuilder<'a, S, BuilderInit> {
        QueryBuilder {
            sql: SQL::empty(),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }
}

impl<'a, Schema> QueryBuilder<'a, Schema, BuilderInit> {
    /// Starts a `SELECT` with the requested projection.
    pub fn select<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), T::Marker>
    where
        T: ToSQL<'a, MySQLValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        QueryBuilder::from_sql(crate::helpers::select(columns))
    }

    /// Starts a `SELECT DISTINCT` with the requested projection.
    pub fn select_distinct<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), T::Marker>
    where
        T: ToSQL<'a, MySQLValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        QueryBuilder::from_sql(crate::helpers::select_distinct(columns))
    }

    /// Starts an `INSERT` for `table`.
    pub fn insert<Table>(
        &self,
        table: Table,
    ) -> insert::InsertBuilder<'a, Schema, insert::InsertInitial, Table>
    where
        Table: MySQLTable<'a>,
    {
        QueryBuilder::from_sql(drizzle_core::helpers::insert::<
            Table,
            MySQLSchemaType,
            MySQLValue<'a>,
        >(&table))
    }

    /// Starts an `UPDATE` for `table`.
    pub fn update<Table>(
        &self,
        table: Table,
    ) -> update::UpdateBuilder<'a, Schema, update::UpdateInitial, Table>
    where
        Table: MySQLTable<'a>,
    {
        QueryBuilder::from_sql(crate::helpers::update::<
            Table,
            MySQLSchemaType,
            MySQLValue<'a>,
        >(&table))
    }

    /// Starts a `DELETE` for `table`.
    pub fn delete<Table>(
        &self,
        table: Table,
    ) -> delete::DeleteBuilder<'a, Schema, delete::DeleteInitial, Table>
    where
        Table: MySQLTable<'a>,
    {
        QueryBuilder::from_sql(crate::helpers::delete::<
            Table,
            MySQLSchemaType,
            MySQLValue<'a>,
        >(&table))
    }

    /// Attaches a common table expression before starting a statement.
    pub fn with<C>(&self, cte: &C) -> QueryBuilder<'a, Schema, CTEInit>
    where
        C: CTEDefinition<'a>,
    {
        QueryBuilder::from_sql(SQL::from(Token::WITH).append(cte.cte_definition()))
    }
}

impl<'a, Schema> QueryBuilder<'a, Schema, CTEInit> {
    /// Starts a `SELECT` after the attached common table expressions.
    pub fn select<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), T::Marker>
    where
        T: ToSQL<'a, MySQLValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        QueryBuilder::from_sql(self.sql.clone().append(crate::helpers::select(columns)))
    }

    /// Starts a `SELECT DISTINCT` after the attached common table expressions.
    pub fn select_distinct<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), T::Marker>
    where
        T: ToSQL<'a, MySQLValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        QueryBuilder::from_sql(
            self.sql
                .clone()
                .append(crate::helpers::select_distinct(columns)),
        )
    }

    /// Starts an `UPDATE` after the attached common table expressions.
    pub fn update<Table>(
        &self,
        table: Table,
    ) -> update::UpdateBuilder<'a, Schema, update::UpdateInitial, Table>
    where
        Table: MySQLTable<'a>,
    {
        QueryBuilder::from_sql(self.sql.clone().append(crate::helpers::update::<
            Table,
            MySQLSchemaType,
            MySQLValue<'a>,
        >(&table)))
    }

    /// Starts a `DELETE` after the attached common table expressions.
    pub fn delete<Table>(
        &self,
        table: Table,
    ) -> delete::DeleteBuilder<'a, Schema, delete::DeleteInitial, Table>
    where
        Table: MySQLTable<'a>,
    {
        QueryBuilder::from_sql(self.sql.clone().append(crate::helpers::delete::<
            Table,
            MySQLSchemaType,
            MySQLValue<'a>,
        >(&table)))
    }

    #[must_use]
    /// Appends another common table expression.
    pub fn with<C>(&self, cte: &C) -> Self
    where
        C: CTEDefinition<'a>,
    {
        QueryBuilder::from_sql(
            self.sql
                .clone()
                .push(Token::COMMA)
                .append(cte.cte_definition()),
        )
    }
}

impl<'a, S, State, T, M, R, G> QueryBuilder<'a, S, State, T, M, R, G> {
    pub(crate) fn from_sql(sql: SQL<'a, MySQLValue<'a>>) -> Self {
        Self {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    pub(crate) fn prepared_statement(
        &self,
    ) -> drizzle_core::prepared::PreparedStatement<'a, MySQLValue<'a>> {
        drizzle_core::prepared::prepare_render(&self.sql)
    }
}
