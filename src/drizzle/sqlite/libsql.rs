use drizzle_core::ParamBind;
use drizzle_core::ToSQL;
use drizzle_core::traits::{IsInSchema, SQLTable};
use libsql::Connection;
use std::marker::PhantomData;

#[cfg(feature = "sqlite")]
use sqlite::{
    SQLiteValue,
    builder::{
        self, QueryBuilder,
        delete::{self, DeleteBuilder},
        insert::{self, InsertBuilder},
        select::{self, SelectBuilder},
        update::{self, UpdateBuilder},
    },
};

use crate::drizzle::sqlite::{DrizzleBuilder, PreparedDrizzle};

/// Drizzle instance that provides access to the database and query builder.
#[derive(Debug)]
pub struct Drizzle<Schema = ()> {
    conn: Connection,
    _schema: PhantomData<Schema>,
}

impl Drizzle {
    pub const fn new<S>(conn: Connection) -> Drizzle<S> {
        Drizzle {
            conn,
            _schema: PhantomData,
        }
    }
}

impl<S> AsRef<Drizzle<S>> for Drizzle<S> {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<Schema> Drizzle<Schema> {
    /// Gets a reference to the underlying connection
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn mut_conn(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Creates a SELECT query builder.
    #[cfg(feature = "sqlite")]
    pub fn select<'a, T>(
        &'a self,
        query: T,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectInitial>,
        select::SelectInitial,
    >
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        use sqlite::builder::QueryBuilder;

        let builder = QueryBuilder::new::<Schema>().select(query);

        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates an INSERT query builder.
    #[cfg(feature = "sqlite")]
    pub fn insert<'a, T>(
        &'a self,
        table: T,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        InsertBuilder<'a, Schema, insert::InsertInitial, T>,
        insert::InsertInitial,
    >
    where
        T: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>> + 'a,
    {
        use sqlite::builder::QueryBuilder;

        let builder = QueryBuilder::new::<Schema>().insert(table);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates an UPDATE query builder.
    #[cfg(feature = "sqlite")]
    pub fn update<'a, T>(
        &'a self,
        table: T,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        UpdateBuilder<'a, Schema, update::UpdateInitial, T>,
        update::UpdateInitial,
    >
    where
        T: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>>,
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
    pub fn delete<'a, T>(
        &'a self,
        table: T,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        DeleteBuilder<'a, Schema, delete::DeleteInitial, T>,
        delete::DeleteInitial,
    >
    where
        T: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>>,
    {
        let builder = QueryBuilder::new::<Schema>().delete(table);
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    pub async fn execute<'a, T>(
        &'a self,
        query: T,
    ) -> Result<u64, drizzle_core::error::DrizzleError>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let query = query.to_sql();
        let sql = query.sql();
        let params: Vec<libsql::Value> = query
            .params()
            .into_iter()
            .map(|p| p.into())
            .collect();

        self.conn
            .execute(&sql, params)
            .await
            .map_err(|e| drizzle_core::error::DrizzleError::Other(e.to_string()))
    }
}

impl<'a, S, State, T> PreparedDrizzle<'a, S, SelectBuilder<'a, S, State, T>, State>
where
    State: builder::ExecutableState,
{
    pub async fn all<R>(
        self,
        params: impl IntoIterator<Item = ParamBind<'a, SQLiteValue<'a>>>,
    ) -> drizzle_core::error::Result<Vec<R>>
    where
        R: for<'r> TryFrom<&'r libsql::Row>,
        for<'r> <R as TryFrom<&'r libsql::Row>>::Error:
            Into<drizzle_core::error::DrizzleError>,
    {
        // Bind parameters to pre-rendered SQL
        let (sql_str, sql_params) = self.sql.bind(params);
        
        // Convert to libsql values
        let libsql_params: Vec<libsql::Value> = sql_params
            .into_iter()
            .map(|p| p.into())
            .collect();

        // Execute with connection
        let conn = &self.drizzle.drizzle.conn;
        let mut rows = conn
            .query(&sql_str, libsql_params)
            .await
            .map_err(|e| drizzle_core::error::DrizzleError::Other(e.to_string()))?;

        let mut results = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| drizzle_core::error::DrizzleError::Other(e.to_string()))?
        {
            let converted = R::try_from(&row).map_err(Into::into)?;
            results.push(converted);
        }

        Ok(results)
    }

    pub async fn get<R>(
        self,
        params: impl IntoIterator<Item = ParamBind<'a, SQLiteValue<'a>>>,
    ) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r libsql::Row>,
        for<'r> <R as TryFrom<&'r libsql::Row>>::Error:
            Into<drizzle_core::error::DrizzleError>,
    {
        // Bind parameters to pre-rendered SQL
        let (sql_str, sql_params) = self.sql.bind(params);
        
        // Convert to libsql values
        let libsql_params: Vec<libsql::Value> = sql_params
            .into_iter()
            .map(|p| p.into())
            .collect();

        // Execute with connection
        let conn = &self.drizzle.drizzle.conn;
        let mut rows = conn
            .query(&sql_str, libsql_params)
            .await
            .map_err(|e| drizzle_core::error::DrizzleError::Other(e.to_string()))?;

        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| drizzle_core::error::DrizzleError::Other(e.to_string()))?
        {
            R::try_from(&row).map_err(Into::into)
        } else {
            Err(drizzle_core::error::DrizzleError::Other("No rows returned".to_string()))
        }
    }
}

// Execution Methods for LibSQL

// Add execution methods for SELECT - LibSQL
impl<'a, S, State, T> DrizzleBuilder<'a, S, SelectBuilder<'a, S, State, T>, State>
where
    State: builder::ExecutableState,
{
    pub async fn all<R>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        R: for<'r> TryFrom<&'r libsql::Row>,
        for<'r> <R as TryFrom<&'r libsql::Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        self.builder.all(&self.drizzle.conn).await
    }

    pub async fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r libsql::Row>,
        for<'r> <R as TryFrom<&'r libsql::Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        self.builder.get(&self.drizzle.conn).await
    }

    pub fn prepare(self) -> PreparedDrizzle<'a, S, SelectBuilder<'a, S, State, T>, State> {
        use drizzle_core::prepare_render;
        let prepared_sql = prepare_render(self.builder.sql.clone());

        PreparedDrizzle {
            drizzle: self,
            sql: prepared_sql,
        }
    }
}

// Add execution methods for INSERT - ValuesSet state - LibSQL
impl<'a, S, T>
    DrizzleBuilder<'a, S, InsertBuilder<'a, S, insert::InsertValuesSet, T>, insert::InsertValuesSet>
{
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        self.builder.execute(&self.drizzle.conn).await
    }
}

// Add execution methods for INSERT - ReturningSet state - LibSQL
impl<'a, S, T>
    DrizzleBuilder<
        'a,
        S,
        InsertBuilder<'a, S, insert::InsertReturningSet, T>,
        insert::InsertReturningSet,
    >
{
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        self.builder.execute(&self.drizzle.conn).await
    }
}

// Add execution methods for INSERT - OnConflictSet state - LibSQL
impl<'a, S, T>
    DrizzleBuilder<
        'a,
        S,
        InsertBuilder<'a, S, insert::InsertOnConflictSet, T>,
        insert::InsertOnConflictSet,
    >
{
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        self.builder.execute(&self.drizzle.conn).await
    }
}

// Add execution methods for UPDATE - SetClauseSet state - LibSQL
impl<'a, S, T>
    DrizzleBuilder<
        'a,
        S,
        UpdateBuilder<'a, S, update::UpdateSetClauseSet, T>,
        update::UpdateSetClauseSet,
    >
{
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        self.builder.execute(&self.drizzle.conn).await
    }
}

// Add execution methods for UPDATE - WhereSet state - LibSQL
impl<'a, S, T>
    DrizzleBuilder<'a, S, UpdateBuilder<'a, S, update::UpdateWhereSet, T>, update::UpdateWhereSet>
{
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        self.builder.execute(&self.drizzle.conn).await
    }
}

// Add execution methods for UPDATE - ReturningSet state - LibSQL
impl<'a, S, T>
    DrizzleBuilder<
        'a,
        S,
        UpdateBuilder<'a, S, update::UpdateReturningSet, T>,
        update::UpdateReturningSet,
    >
{
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        self.builder.execute(&self.drizzle.conn).await
    }
}

// Add execution methods for DELETE - Initial state - LibSQL
impl<'a, S, T>
    DrizzleBuilder<'a, S, DeleteBuilder<'a, S, delete::DeleteInitial, T>, delete::DeleteInitial>
{
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        self.builder.execute(&self.drizzle.conn).await
    }
}

// Add execution methods for DELETE - WhereSet state - LibSQL
impl<'a, S, T>
    DrizzleBuilder<'a, S, DeleteBuilder<'a, S, delete::DeleteWhereSet, T>, delete::DeleteWhereSet>
{
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        self.builder.execute(&self.drizzle.conn).await
    }
}

// Add execution methods for DELETE - ReturningSet state - LibSQL
impl<'a, S, T>
    DrizzleBuilder<
        'a,
        S,
        DeleteBuilder<'a, S, delete::DeleteReturningSet, T>,
        delete::DeleteReturningSet,
    >
{
    pub async fn execute(self) -> drizzle_core::error::Result<u64> {
        self.builder.execute(&self.drizzle.conn).await
    }
}