//! Type-safe MySQL query construction.

use crate::{common::MySQLSchemaType, traits::MySQLTable, values::MySQLValue};
use core::{fmt::Debug, marker::PhantomData};
use drizzle_core::{SQL, ToSQL, Token};

pub use drizzle_core::{BuilderInit, ExecutableState};

pub mod cte;
pub mod delete;
pub mod insert;
pub mod select;
pub mod update;

pub use cte::{CTEDefinition, CTEView};
pub use delete::{DeleteBuilder, DeleteInitial, DeleteLimitSet, DeleteOrderSet, DeleteWhereSet};
pub use insert::{InsertBuilder, InsertInitial, InsertValuesSet};
pub use select::{
    IntoSelect, SelectBuilder, SelectFromSet, SelectGroupSet, SelectHavingSet, SelectInitial,
    SelectJoinSet, SelectLimitSet, SelectOffsetSet, SelectOrderSet, SelectSetOpSet, SelectWhereSet,
};
pub use update::{
    UpdateBuilder, UpdateInitial, UpdateLimitSet, UpdateOrderSet, UpdateSetClauseSet,
    UpdateWhereSet,
};

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
    #[must_use]
    pub fn comment(mut self, text: impl AsRef<str>) -> Self {
        let fragment = drizzle_core::sql::comment::<MySQLValue<'a>>(text);
        if !fragment.chunks.is_empty() {
            let existing = core::mem::replace(&mut self.sql, fragment);
            self.sql.append_mut(existing);
        }
        self
    }

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

fn select_builder<'a, S, T>(
    sql: SQL<'a, MySQLValue<'a>>,
) -> select::SelectBuilder<'a, S, select::SelectInitial, (), T::Marker>
where
    T: drizzle_core::IntoSelectTarget,
{
    QueryBuilder {
        sql,
        schema: PhantomData,
        state: PhantomData,
        table: PhantomData,
        marker: PhantomData,
        row: PhantomData,
        grouped: PhantomData,
    }
}

impl<'a, Schema> QueryBuilder<'a, Schema, BuilderInit> {
    pub fn select<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), T::Marker>
    where
        T: ToSQL<'a, MySQLValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        select_builder::<Schema, T>(crate::helpers::select(columns))
    }

    pub fn select_distinct<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), T::Marker>
    where
        T: ToSQL<'a, MySQLValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        select_builder::<Schema, T>(crate::helpers::select_distinct(columns))
    }

    pub fn insert<Table>(
        &self,
        table: Table,
    ) -> insert::InsertBuilder<'a, Schema, insert::InsertInitial, Table>
    where
        Table: MySQLTable<'a>,
    {
        QueryBuilder::from_sql(crate::helpers::insert(&table))
    }

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

    pub fn with<C>(&self, cte: &C) -> QueryBuilder<'a, Schema, CTEInit>
    where
        C: CTEDefinition<'a>,
    {
        QueryBuilder::from_sql(SQL::from(Token::WITH).append(cte.cte_definition()))
    }
}

impl<'a, Schema> QueryBuilder<'a, Schema, CTEInit> {
    pub fn select<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), T::Marker>
    where
        T: ToSQL<'a, MySQLValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        select_builder::<Schema, T>(self.sql.clone().append(crate::helpers::select(columns)))
    }

    pub fn select_distinct<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), T::Marker>
    where
        T: ToSQL<'a, MySQLValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        select_builder::<Schema, T>(
            self.sql
                .clone()
                .append(crate::helpers::select_distinct(columns)),
        )
    }

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
