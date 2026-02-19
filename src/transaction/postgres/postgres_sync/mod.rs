use drizzle_core::error::DrizzleError;
use drizzle_core::traits::ToSQL;
use drizzle_postgres::builder::{DeleteInitial, InsertInitial, SelectInitial, UpdateInitial};
use drizzle_postgres::traits::PostgresTable;
use postgres::fallible_iterator::FallibleIterator;
use postgres::{Row, Transaction as PgTransaction};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, Ordering};

pub mod delete;
pub mod insert;
pub mod select;
pub mod update;

use drizzle_postgres::builder::{
    self, QueryBuilder, delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder,
    update::UpdateBuilder,
};
use drizzle_postgres::common::PostgresTransactionType;
use drizzle_postgres::values::PostgresValue;
use smallvec::SmallVec;

use crate::builder::postgres::postgres_sync::Rows;

/// Postgres-specific transaction builder
#[derive(Debug)]
pub struct TransactionBuilder<'a, 'conn, Schema, Builder, State> {
    transaction: &'a Transaction<'conn, Schema>,
    builder: Builder,
    _phantom: PhantomData<(Schema, State)>,
}

/// Transaction wrapper that provides the same query building capabilities as Drizzle
pub struct Transaction<'conn, Schema = ()> {
    tx: RefCell<Option<PgTransaction<'conn>>>,
    tx_type: PostgresTransactionType,
    savepoint_depth: AtomicU32,
    schema: Schema,
}

impl<'conn, Schema> std::fmt::Debug for Transaction<'conn, Schema> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction")
            .field("tx_type", &self.tx_type)
            .field("is_active", &self.tx.borrow().is_some())
            .finish()
    }
}

impl<'conn, Schema> Transaction<'conn, Schema> {
    /// Creates a new transaction wrapper
    pub(crate) fn new(
        tx: PgTransaction<'conn>,
        tx_type: PostgresTransactionType,
        schema: Schema,
    ) -> Self {
        Self {
            tx: RefCell::new(Some(tx)),
            tx_type,
            savepoint_depth: AtomicU32::new(0),
            schema,
        }
    }

    /// Gets a reference to the schema.
    #[inline]
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Gets the transaction type
    #[inline]
    pub fn tx_type(&self) -> PostgresTransactionType {
        self.tx_type
    }

    /// Executes a raw SQL string with no parameters.
    fn execute_raw(&self, sql: &str) -> drizzle_core::error::Result<()> {
        let mut tx_ref = self.tx.borrow_mut();
        let tx = tx_ref.as_mut().expect("Transaction already consumed");
        tx.execute(sql, &[])
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
        Ok(())
    }

    /// Executes a nested savepoint within this transaction.
    ///
    /// The callback receives a reference to this transaction for executing
    /// queries. If the callback returns `Ok`, the savepoint is released.
    /// If it returns `Err` or panics, the savepoint is rolled back.
    /// The outer transaction is unaffected either way.
    ///
    /// Savepoints can be nested — each level gets its own savepoint name.
    ///
    /// ```no_run
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::sync::Drizzle;
    /// # use drizzle::postgres::common::PostgresTransactionType;
    /// # #[PostgresTable] struct User { #[column(serial, primary)] id: i32, name: String }
    /// # #[derive(PostgresSchema)] struct S { user: User }
    /// # fn main() -> drizzle::Result<()> {
    /// # let client = ::postgres::Client::connect("host=localhost user=postgres", ::postgres::NoTls)?;
    /// # let (mut db, S { user }) = Drizzle::new(client, S::new());
    /// db.transaction(PostgresTransactionType::ReadCommitted, |tx| {
    ///     tx.insert(user).values([InsertUser::new("Alice")]).execute()?;
    ///
    ///     // This savepoint fails — only its changes roll back
    ///     let _: Result<(), _> = tx.savepoint(|stx| {
    ///         stx.insert(user).values([InsertUser::new("Bad")]).execute()?;
    ///         Err(drizzle::error::DrizzleError::Other("oops".into()))
    ///     });
    ///
    ///     let users: Vec<SelectUser> = tx.select(()).from(user).all()?;
    ///     assert_eq!(users.len(), 1); // only Alice
    ///     Ok(())
    /// })?;
    /// # Ok(()) }
    /// ```
    pub fn savepoint<F, R>(&self, f: F) -> drizzle_core::error::Result<R>
    where
        F: FnOnce(&Self) -> drizzle_core::error::Result<R>,
    {
        let depth = self.savepoint_depth.load(Ordering::Relaxed);
        let sp_name = format!("drizzle_sp_{}", depth);
        self.savepoint_depth.store(depth + 1, Ordering::Relaxed);

        self.execute_raw(&format!("SAVEPOINT {}", sp_name))?;

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(self)));

        self.savepoint_depth.store(depth, Ordering::Relaxed);

        match result {
            Ok(Ok(value)) => {
                self.execute_raw(&format!("RELEASE SAVEPOINT {}", sp_name))?;
                Ok(value)
            }
            Ok(Err(e)) => {
                let _ = self.execute_raw(&format!("ROLLBACK TO SAVEPOINT {}", sp_name));
                let _ = self.execute_raw(&format!("RELEASE SAVEPOINT {}", sp_name));
                Err(e)
            }
            Err(panic_payload) => {
                let _ = self.execute_raw(&format!("ROLLBACK TO SAVEPOINT {}", sp_name));
                let _ = self.execute_raw(&format!("RELEASE SAVEPOINT {}", sp_name));
                std::panic::resume_unwind(panic_payload);
            }
        }
    }

    /// Creates a SELECT query builder within the transaction
    pub fn select<'a, 'b, T>(
        &'a self,
        query: T,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'b, Schema, SelectInitial, (), T::Marker>,
        SelectInitial,
    >
    where
        T: ToSQL<'b, PostgresValue<'b>> + drizzle_core::IntoSelectTarget,
    {
        use drizzle_postgres::builder::QueryBuilder;

        let builder = QueryBuilder::new::<Schema>().select(query);

        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Creates an INSERT query builder within the transaction
    pub fn insert<'a, Table>(
        &'a self,
        table: Table,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        InsertBuilder<'a, Schema, InsertInitial, Table>,
        InsertInitial,
    >
    where
        Table: PostgresTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().insert(table);
        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Creates an UPDATE query builder within the transaction
    pub fn update<'a, Table>(
        &'a self,
        table: Table,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        UpdateBuilder<'a, Schema, UpdateInitial, Table>,
        UpdateInitial,
    >
    where
        Table: PostgresTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().update(table);
        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Creates a DELETE query builder within the transaction
    pub fn delete<'a, T>(
        &'a self,
        table: T,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        DeleteBuilder<'a, Schema, DeleteInitial, T>,
        DeleteInitial,
    >
    where
        T: PostgresTable<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().delete(table);
        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    /// Creates a query with CTE (Common Table Expression) within the transaction
    pub fn with<'a, C>(
        &'a self,
        cte: C,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        QueryBuilder<'a, Schema, builder::CTEInit>,
        builder::CTEInit,
    >
    where
        C: builder::CTEDefinition<'a>,
    {
        let builder = QueryBuilder::new::<Schema>().with(cte);
        TransactionBuilder {
            transaction: self,
            builder,
            _phantom: PhantomData,
        }
    }

    pub fn execute<'a, T>(&'a self, query: T) -> Result<u64, postgres::Error>
    where
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx.execute");
        let query_sql = query.to_sql();
        let (sql, params) = query_sql.build();
        drizzle_core::drizzle_trace_query!(&sql, params.len());

        let mut tx_ref = self.tx.borrow_mut();
        let tx = tx_ref.as_mut().expect("Transaction already consumed");

        let param_refs = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.sync", "tx.execute.param_refs");
            let mut param_refs: SmallVec<[&(dyn postgres::types::ToSql + Sync); 8]> =
                SmallVec::with_capacity(params.len());
            param_refs.extend(
                params
                    .iter()
                    .map(|&p| p as &(dyn postgres::types::ToSql + Sync)),
            );
            param_refs
        };

        let mut typed_params: SmallVec<
            [(&(dyn postgres::types::ToSql + Sync), postgres::types::Type); 8],
        > = SmallVec::with_capacity(params.len());
        let mut all_typed = true;
        for p in &params {
            if let Some(ty) = crate::builder::postgres::prepared_common::postgres_sync_param_type(p)
            {
                typed_params.push((*p as &(dyn postgres::types::ToSql + Sync), ty));
            } else {
                all_typed = false;
                break;
            }
        }

        if all_typed {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.sync", "tx.execute.db_typed");
            let mut rows = tx.query_typed_raw(&sql, typed_params)?;
            while rows.next()?.is_some() {}
            return Ok(rows.rows_affected().unwrap_or(0));
        }

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx.execute.db");
        tx.execute(&sql, &param_refs[..])
    }

    /// Runs the query and returns all matching rows (for SELECT queries)
    pub fn all<'a, T, R, C>(&'a self, query: T) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
        C: std::iter::FromIterator<R>,
    {
        self.rows(query)?
            .collect::<drizzle_core::error::Result<C>>()
    }

    /// Runs the query and returns a lazy row cursor.
    pub fn rows<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx.all");
        let sql = query.to_sql();
        let (sql_str, params) = sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx.all.param_refs");
        let mut param_refs: SmallVec<[&(dyn postgres::types::ToSql + Sync); 8]> =
            SmallVec::with_capacity(params.len());
        param_refs.extend(
            params
                .iter()
                .map(|&p| p as &(dyn postgres::types::ToSql + Sync)),
        );

        let mut tx_ref = self.tx.borrow_mut();
        let tx = tx_ref.as_mut().expect("Transaction already consumed");

        let rows = tx
            .query(&sql_str, &param_refs[..])
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        Ok(Rows::new(rows))
    }

    /// Runs the query and returns a single row (for SELECT queries)
    pub fn get<'a, T, R>(&'a self, query: T) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx.get");
        let sql = query.to_sql();
        let (sql_str, params) = sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx.get.param_refs");
        let mut param_refs: SmallVec<[&(dyn postgres::types::ToSql + Sync); 8]> =
            SmallVec::with_capacity(params.len());
        param_refs.extend(
            params
                .iter()
                .map(|&p| p as &(dyn postgres::types::ToSql + Sync)),
        );

        let mut tx_ref = self.tx.borrow_mut();
        let tx = tx_ref.as_mut().expect("Transaction already consumed");

        let row = tx
            .query_one(&sql_str, &param_refs[..])
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        R::try_from(&row).map_err(Into::into)
    }

    /// Commits the transaction
    pub(crate) fn commit(&self) -> drizzle_core::error::Result<()> {
        let tx = self
            .tx
            .borrow_mut()
            .take()
            .expect("Transaction already consumed");
        tx.commit()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))
    }

    /// Rolls back the transaction
    pub(crate) fn rollback(&self) -> drizzle_core::error::Result<()> {
        let tx = self
            .tx
            .borrow_mut()
            .take()
            .expect("Transaction already consumed");
        tx.rollback()
            .map_err(|e| DrizzleError::Other(e.to_string().into()))
    }
}

impl<'a, 'conn, Schema>
    TransactionBuilder<
        'a,
        'conn,
        Schema,
        QueryBuilder<'a, Schema, builder::CTEInit>,
        builder::CTEInit,
    >
{
    #[inline]
    pub fn select<T>(
        self,
        query: T,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        SelectBuilder<'a, Schema, SelectInitial, (), T::Marker>,
        SelectInitial,
    >
    where
        T: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        let builder = self.builder.select(query);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub fn with<C>(
        self,
        cte: C,
    ) -> TransactionBuilder<
        'a,
        'conn,
        Schema,
        QueryBuilder<'a, Schema, builder::CTEInit>,
        builder::CTEInit,
    >
    where
        C: builder::CTEDefinition<'a>,
    {
        let builder = self.builder.with(cte);
        TransactionBuilder {
            transaction: self.transaction,
            builder,
            _phantom: PhantomData,
        }
    }
}

impl<'a, 'conn, S, Schema, State, Table, Mk, Rw>
    TransactionBuilder<'a, 'conn, S, QueryBuilder<'a, Schema, State, Table, Mk, Rw>, State>
where
    State: builder::ExecutableState,
{
    /// Runs the query and returns the number of affected rows
    pub fn execute(self) -> drizzle_core::error::Result<u64> {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.execute");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        let mut tx_ref = self.transaction.tx.borrow_mut();
        let tx = tx_ref.as_mut().expect("Transaction already consumed");

        let param_refs = {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.execute.param_refs");
            let mut param_refs: SmallVec<[&(dyn postgres::types::ToSql + Sync); 8]> =
                SmallVec::with_capacity(params.len());
            param_refs.extend(
                params
                    .iter()
                    .map(|&p| p as &(dyn postgres::types::ToSql + Sync)),
            );
            param_refs
        };

        let mut typed_params: SmallVec<
            [(&(dyn postgres::types::ToSql + Sync), postgres::types::Type); 8],
        > = SmallVec::with_capacity(params.len());
        let mut all_typed = true;
        for p in &params {
            if let Some(ty) = crate::builder::postgres::prepared_common::postgres_sync_param_type(p)
            {
                typed_params.push((*p as &(dyn postgres::types::ToSql + Sync), ty));
            } else {
                all_typed = false;
                break;
            }
        }

        if all_typed {
            #[cfg(feature = "profiling")]
            drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.execute.db_typed");
            let mut rows = tx
                .query_typed_raw(&sql_str, typed_params)
                .map_err(|e| DrizzleError::Other(e.to_string().into()))?;
            while rows
                .next()
                .map_err(|e| DrizzleError::Other(e.to_string().into()))?
                .is_some()
            {}
            return Ok(rows.rows_affected().unwrap_or(0));
        }

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.execute.db");
        Ok(tx
            .execute(&sql_str, &param_refs[..])
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?)
    }

    /// Runs the query and returns all matching rows, decoded as the given type `R`.
    pub fn all_as<R, C>(self) -> drizzle_core::error::Result<C>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
        C: FromIterator<R>,
    {
        self.rows_as::<R>()?
            .collect::<drizzle_core::error::Result<C>>()
    }

    /// Runs the query and returns a lazy row cursor, decoded as the given type `R`.
    pub fn rows_as<R>(self) -> drizzle_core::error::Result<Rows<R>>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.all");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.all.param_refs");
        let mut param_refs: SmallVec<[&(dyn postgres::types::ToSql + Sync); 8]> =
            SmallVec::with_capacity(params.len());
        param_refs.extend(
            params
                .iter()
                .map(|&p| p as &(dyn postgres::types::ToSql + Sync)),
        );

        let mut tx_ref = self.transaction.tx.borrow_mut();
        let tx = tx_ref.as_mut().expect("Transaction already consumed");

        let rows = tx
            .query(&sql_str, &param_refs[..])
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        Ok(Rows::new(rows))
    }

    /// Runs the query and returns a single row, decoded as the given type `R`.
    pub fn get_as<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r Row>,
        for<'r> <R as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.get");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.get.param_refs");
        let mut param_refs: SmallVec<[&(dyn postgres::types::ToSql + Sync); 8]> =
            SmallVec::with_capacity(params.len());
        param_refs.extend(
            params
                .iter()
                .map(|&p| p as &(dyn postgres::types::ToSql + Sync)),
        );

        let mut tx_ref = self.transaction.tx.borrow_mut();
        let tx = tx_ref.as_mut().expect("Transaction already consumed");

        let row = tx
            .query_one(&sql_str, &param_refs[..])
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        R::try_from(&row).map_err(Into::into)
    }

    /// Runs the query and returns all matching rows using the builder's row type.
    pub fn all<R, Proof>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::postgres::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::postgres::Row, Rw, R>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.all");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.all.param_refs");
        let mut param_refs: SmallVec<[&(dyn postgres::types::ToSql + Sync); 8]> =
            SmallVec::with_capacity(params.len());
        param_refs.extend(
            params
                .iter()
                .map(|&p| p as &(dyn postgres::types::ToSql + Sync)),
        );

        let mut tx_ref = self.transaction.tx.borrow_mut();
        let tx = tx_ref.as_mut().expect("Transaction already consumed");
        let rows = tx
            .query(&sql_str, &param_refs[..])
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        let mut decoded = Vec::with_capacity(rows.len());
        for row in &rows {
            decoded.push(<Mk as drizzle_core::row::DecodeSelectedRef<
                &::postgres::Row,
                R,
            >>::decode(row)?);
        }
        Ok(decoded)
    }

    /// Runs the query and returns a lazy row cursor using the builder's row type.
    pub fn rows(self) -> drizzle_core::error::Result<Rows<Rw>>
    where
        Rw: for<'r> TryFrom<&'r Row>,
        for<'r> <Rw as TryFrom<&'r Row>>::Error: Into<drizzle_core::error::DrizzleError>,
    {
        self.rows_as()
    }

    /// Runs the query and returns a single row using the builder's row type.
    pub fn get<R, Proof>(self) -> drizzle_core::error::Result<R>
    where
        for<'r> Mk: drizzle_core::row::DecodeSelectedRef<&'r ::postgres::Row, R>
            + drizzle_core::row::MarkerScopeValidFor<Proof>
            + drizzle_core::row::StrictDecodeMarker
            + drizzle_core::row::MarkerColumnCountValid<::postgres::Row, Rw, R>,
    {
        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.get");
        let (sql_str, params) = self.builder.sql.build();
        drizzle_core::drizzle_trace_query!(&sql_str, params.len());

        #[cfg(feature = "profiling")]
        drizzle_core::drizzle_profile_scope!("postgres.sync", "tx_builder.get.param_refs");
        let mut param_refs: SmallVec<[&(dyn postgres::types::ToSql + Sync); 8]> =
            SmallVec::with_capacity(params.len());
        param_refs.extend(
            params
                .iter()
                .map(|&p| p as &(dyn postgres::types::ToSql + Sync)),
        );

        let mut tx_ref = self.transaction.tx.borrow_mut();
        let tx = tx_ref.as_mut().expect("Transaction already consumed");
        let row = tx
            .query_one(&sql_str, &param_refs[..])
            .map_err(|e| DrizzleError::Other(e.to_string().into()))?;

        <Mk as drizzle_core::row::DecodeSelectedRef<&::postgres::Row, R>>::decode(&row)
    }
}

impl<'a, 'conn, S, T, State> ToSQL<'a, PostgresValue<'a>>
    for TransactionBuilder<'a, 'conn, S, T, State>
where
    T: ToSQL<'a, PostgresValue<'a>>,
{
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, PostgresValue<'a>> {
        self.builder.to_sql()
    }
}
