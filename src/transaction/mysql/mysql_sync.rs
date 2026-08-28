//! Blocking MySQL transactions.

use core::sync::atomic::AtomicU32;
use std::cell::{Cell, RefCell};

use drizzle_core::{
    error::{DrizzleError, QueryContext, Result, ResultExt},
    row::{
        DecodeSelectedRef, FromDrizzleRow, MarkerAggValidFor, MarkerColumnCountValid,
        MarkerScopeValidFor, StrictDecodeMarker,
    },
    traits::ToSQL,
};
use drizzle_mysql::{
    AccessMode, IsolationLevel, MySQLMutationResult, MySQLRow, TransactionConfig,
    builder::{
        self, DeleteBuilder, DeleteInitial, InsertBuilder, InsertInitial, QueryBuilder,
        SelectBuilder, SelectInitial, UpdateBuilder, UpdateInitial,
    },
    traits::MySQLTable,
    values::MySQLValue,
};
use mysql::{
    AccessMode as DriverAccessMode, IsolationLevel as DriverIsolationLevel, Row,
    Transaction as DriverTransaction, TxOpts, prelude::Queryable,
};

use crate::{
    builder::mysql::{
        common::{self, DrizzleBuilder},
        driver_common::{QueryOutput, render},
        mysql_sync::{
            Rows, driver_error, execute_request_observing, initialize_session_observing,
            query_first_request_observing, query_request_observing,
        },
    },
    transaction::savepoint::sync_savepoint,
};

fn consumed() -> DrizzleError {
    DrizzleError::TransactionError("MySQL transaction already consumed".into())
}

fn aborted() -> DrizzleError {
    DrizzleError::TransactionError(
        "MySQL transaction is unusable after the server aborted it".into(),
    )
}

fn transaction_was_aborted(error: &mysql::Error) -> bool {
    error.is_connectivity_error()
        || matches!(error, mysql::Error::MySqlError(error) if error.code == 1205 || error.state.starts_with("40"))
}

pub(crate) fn options(config: TransactionConfig) -> TxOpts {
    let isolation = config.isolation().map(|level| match level {
        IsolationLevel::ReadUncommitted => DriverIsolationLevel::ReadUncommitted,
        IsolationLevel::ReadCommitted => DriverIsolationLevel::ReadCommitted,
        IsolationLevel::RepeatableRead => DriverIsolationLevel::RepeatableRead,
        IsolationLevel::Serializable => DriverIsolationLevel::Serializable,
    });
    let access = config.access().map(|mode| match mode {
        AccessMode::ReadOnly => DriverAccessMode::ReadOnly,
        AccessMode::ReadWrite => DriverAccessMode::ReadWrite,
    });
    TxOpts::default()
        .set_isolation_level(isolation)
        .set_access_mode(access)
        .set_with_consistent_snapshot(config.consistent_snapshot())
}

pub(crate) trait TransactionConnection: Queryable {
    fn start_drizzle_transaction(
        &mut self,
        options: TxOpts,
    ) -> mysql::Result<DriverTransaction<'_>>;
}

impl TransactionConnection for mysql::Conn {
    fn start_drizzle_transaction(
        &mut self,
        options: TxOpts,
    ) -> mysql::Result<DriverTransaction<'_>> {
        self.start_transaction(options)
    }
}

impl TransactionConnection for mysql::PooledConn {
    fn start_drizzle_transaction(
        &mut self,
        options: TxOpts,
    ) -> mysql::Result<DriverTransaction<'_>> {
        self.start_transaction(options)
    }
}

pub(crate) fn start_transaction<C: TransactionConnection>(
    connection: &mut C,
    config: TransactionConfig,
) -> mysql::Result<DriverTransaction<'_>> {
    if !config.consistent_snapshot() {
        return connection.start_drizzle_transaction(options(config));
    }

    // mysql 28.0.0 unwraps the server result for WITH CONSISTENT SNAPSHOT.
    // Obtain its RAII transaction wrapper first, return that temporary
    // transaction to an idle state, then issue the real transaction setup
    // through fallible Queryable calls.
    let mut transaction = connection.start_drizzle_transaction(TxOpts::default())?;
    transaction.query_drop("ROLLBACK")?;
    if let Some(isolation) = config.isolation() {
        transaction.query_drop(format!("SET TRANSACTION ISOLATION LEVEL {isolation}"))?;
    }
    if let Some(access) = config.access() {
        transaction.query_drop(format!("SET TRANSACTION {access}"))?;
    }
    transaction.query_drop("START TRANSACTION WITH CONSISTENT SNAPSHOT")?;
    Ok(transaction)
}

/// A scoped blocking MySQL transaction.
///
/// Dropping an active value delegates rollback to the upstream driver.
pub struct Transaction<'connection, Schema = ()> {
    transaction: RefCell<Option<DriverTransaction<'connection>>>,
    schema: Schema,
    savepoint_depth: AtomicU32,
    poisoned: Cell<bool>,
    session_ready: Cell<bool>,
}

impl<Schema> core::fmt::Debug for Transaction<'_, Schema> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("Transaction")
            .field("active", &self.transaction.borrow().is_some())
            .field("poisoned", &self.poisoned.get())
            .finish_non_exhaustive()
    }
}

impl<'connection, Schema> Transaction<'connection, Schema> {
    pub(crate) fn new(transaction: DriverTransaction<'connection>, schema: Schema) -> Self {
        Self {
            transaction: RefCell::new(Some(transaction)),
            schema,
            savepoint_depth: AtomicU32::new(0),
            poisoned: Cell::new(false),
            session_ready: Cell::new(true),
        }
    }

    fn ensure_usable(&self) -> Result<()> {
        if self.poisoned.get() {
            Err(aborted())
        } else {
            Ok(())
        }
    }

    fn observe_error(&self, error: &mysql::Error) {
        if transaction_was_aborted(error) {
            self.poisoned.set(true);
        }
    }

    fn ensure_session(&self) -> Result<()> {
        self.ensure_usable()?;
        if self.session_ready.get() {
            return Ok(());
        }

        let mut transaction = self.transaction.borrow_mut();
        initialize_session_observing(transaction.as_mut().ok_or_else(consumed)?, |error| {
            self.observe_error(error)
        })?;
        self.session_ready.set(true);
        Ok(())
    }

    /// Borrows the schema attached to this transaction.
    #[must_use]
    pub const fn schema(&self) -> &Schema {
        &self.schema
    }

    pub(crate) fn execute_rendered<'q>(
        &self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<MySQLMutationResult> {
        self.ensure_session()?;
        let (sql, values) = render(query);
        let mut transaction = self.transaction.borrow_mut();
        execute_request_observing(
            transaction.as_mut().ok_or_else(consumed)?,
            &sql,
            &values,
            |error| self.observe_error(error),
        )
    }

    pub(crate) fn query_rendered<'q>(
        &self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<QueryOutput<'q>> {
        self.ensure_session()?;
        let (sql, values) = render(query);
        let mut transaction = self.transaction.borrow_mut();
        let rows = query_request_observing(
            transaction.as_mut().ok_or_else(consumed)?,
            &sql,
            &values,
            |error| self.observe_error(error),
        )?;
        Ok(QueryOutput::new(sql, values, rows))
    }

    pub(crate) fn query_first_rendered<'q>(
        &self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<QueryOutput<'q>> {
        self.ensure_session()?;
        let (sql, values) = render(query);
        let mut transaction = self.transaction.borrow_mut();
        let rows = query_first_request_observing(
            transaction.as_mut().ok_or_else(consumed)?,
            &sql,
            &values,
            |error| self.observe_error(error),
        )?
        .into_iter()
        .collect();
        Ok(QueryOutput::new(sql, values, rows))
    }

    fn execute_raw(&self, sql: &str) -> Result<()> {
        self.ensure_usable()?;
        let mut transaction = self.transaction.borrow_mut();
        drizzle_core::drizzle_trace_query!(sql, 0);
        transaction
            .as_mut()
            .ok_or_else(consumed)?
            .query_drop(sql)
            .map_err(|error| {
                self.poisoned.set(true);
                driver_error(error)
            })
            .with_query(|| QueryContext::new::<MySQLValue<'_>>(sql, &[]))
    }

    /// Commits this transaction. A second completion attempt is an error.
    pub(crate) fn commit(self) -> Result<()> {
        let transaction = self.transaction.borrow_mut().take().ok_or_else(consumed)?;
        if self.poisoned.get() {
            return match transaction.rollback() {
                Ok(()) => Err(aborted()),
                Err(error) => Err(DrizzleError::TransactionError(
                    format!("{}; rollback failed: {error}", aborted()).into(),
                )),
            };
        }
        transaction
            .commit()
            .map_err(crate::builder::mysql::mysql_sync::driver_error)
    }

    /// Rolls this transaction back. A second completion attempt is an error.
    pub(crate) fn rollback(self) -> Result<()> {
        self.transaction
            .borrow_mut()
            .take()
            .ok_or_else(consumed)?
            .rollback()
            .map_err(crate::builder::mysql::mysql_sync::driver_error)
    }

    /// Runs a nested unit using a MySQL savepoint.
    ///
    /// Returning `Ok` releases the savepoint. Returning `Err` rolls it back
    /// without ending the surrounding transaction.
    ///
    /// # Errors
    ///
    /// Returns the callback error, or an error from savepoint creation,
    /// release, or rollback.
    pub fn savepoint<R>(&self, body: impl FnOnce(&Self) -> Result<R>) -> Result<R> {
        self.ensure_usable()?;
        sync_savepoint(
            &self.savepoint_depth,
            |sql| self.execute_raw(sql),
            || body(self),
        )
    }

    /// Executes arbitrary typed MySQL SQL through the prepared/binary protocol.
    ///
    /// The result contains affected-row and last-insert-ID metadata from the
    /// server's OK packet.
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction is unusable or execution fails.
    pub fn execute<'q>(
        &self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<MySQLMutationResult> {
        let result = self.execute_rendered(query);
        // Arbitrary SQL can change the session settings owned by this adapter.
        self.session_ready.set(false);
        result
    }

    /// Executes typed MySQL SQL and decodes every returned row.
    ///
    /// # Errors
    ///
    /// Returns an error if execution or row decoding fails.
    pub fn all<'q, R>(&self, query: impl ToSQL<'q, MySQLValue<'q>>) -> Result<Vec<R>>
    where
        for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        self.rows::<_, R>(query)?.collect()
    }

    /// Executes typed MySQL SQL and returns a decoded iterator over its
    /// materialized rows.
    ///
    /// The database result is fully consumed before this method returns.
    ///
    /// # Errors
    ///
    /// Returns an error if execution or row decoding fails.
    pub fn rows<'q, T, R>(&self, query: T) -> Result<Rows<R>>
    where
        T: ToSQL<'q, MySQLValue<'q>>,
        for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        Ok(self.query_rendered(query)?.rows::<R>())
    }

    /// Executes typed MySQL SQL and decodes the first row.
    ///
    /// # Errors
    ///
    /// Returns an error if execution or decoding fails, or no row is returned.
    pub fn get<'q, R>(&self, query: impl ToSQL<'q, MySQLValue<'q>>) -> Result<R>
    where
        for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        self.query_first_rendered(query)?.decode_first_row()
    }

    /// Creates a typed relational query scoped to this transaction.
    #[cfg(feature = "query")]
    pub fn query<'db, 'q, Table>(
        &'db self,
        _table: Table,
    ) -> common::DrizzleQueryBuilder<'db, 'q, &'db Self, Schema, Table>
    where
        Table: drizzle_core::query::QueryTable,
    {
        common::DrizzleQueryBuilder {
            runner: self,
            builder: drizzle_core::query::QueryBuilder::new(),
            state: core::marker::PhantomData,
        }
    }

    mysql_builder_constructors!(&'db Transaction<'connection, Schema>, [&'db self], self);
}

#[cfg(feature = "query")]
impl<Schema> common::RelationalPreparedDriver for &Transaction<'_, Schema> {
    type PreparedDriver = crate::builder::mysql::mysql_sync::RelationalPrepared;
}

#[cfg(feature = "query")]
impl<'db, 'connection, 'q, Schema, Table, Relations, Clauses>
    common::DrizzleQueryBuilder<
        'db,
        'q,
        &'db Transaction<'connection, Schema>,
        Schema,
        Table,
        Relations,
        drizzle_core::query::AllColumns,
        Clauses,
    >
{
    /// Executes the relational query and decodes every full row.
    ///
    /// # Errors
    ///
    /// Returns an error if execution or relational row decoding fails.
    pub fn find_many(
        self,
    ) -> Result<Vec<<Relations as drizzle_core::query::BuildRow<Table::Select>>::Row>>
    where
        Table: drizzle_core::query::QueryTable,
        for<'row> Table::Select: FromDrizzleRow<MySQLRow<'row, Row>>,
        Relations: drizzle_core::query::BuildRow<Table::Select>
            + drizzle_core::query::RenderRelations<'q, MySQLValue<'q>>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        self.runner
            .query_rendered(common::render_relational_all(self.builder))?
            .decode_relational_all::<Table, Relations>()
    }
}

#[cfg(feature = "query")]
impl<'db, 'connection, 'q, Schema, Table, Relations, Where, Order>
    common::DrizzleQueryBuilder<
        'db,
        'q,
        &'db Transaction<'connection, Schema>,
        Schema,
        Table,
        Relations,
        drizzle_core::query::AllColumns,
        drizzle_core::query::Clauses<Where, Order, drizzle_core::query::NoLimit>,
    >
{
    /// Executes the relational query with a one-row limit.
    ///
    /// # Errors
    ///
    /// Returns an error if execution or relational row decoding fails.
    pub fn find_first(
        self,
    ) -> Result<Option<<Relations as drizzle_core::query::BuildRow<Table::Select>>::Row>>
    where
        Table: drizzle_core::query::QueryTable,
        for<'row> Table::Select: FromDrizzleRow<MySQLRow<'row, Row>>,
        Relations: drizzle_core::query::BuildRow<Table::Select>
            + drizzle_core::query::RenderRelations<'q, MySQLValue<'q>>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many()?.into_iter().next())
    }
}

#[cfg(feature = "query")]
impl<'db, 'connection, 'q, Schema, Table, Relations, Clauses>
    common::DrizzleQueryBuilder<
        'db,
        'q,
        &'db Transaction<'connection, Schema>,
        Schema,
        Table,
        Relations,
        drizzle_core::query::PartialColumns,
        Clauses,
    >
{
    /// Executes the relational query and decodes every partial row.
    ///
    /// # Errors
    ///
    /// Returns an error if execution or relational row decoding fails.
    pub fn find_many(
        self,
    ) -> Result<Vec<<Relations as drizzle_core::query::BuildRow<Table::PartialSelect>>::Row>>
    where
        Table: drizzle_core::query::QueryTable,
        Table::PartialSelect: drizzle_core::query::FromJsonObject,
        Relations: drizzle_core::query::BuildRow<Table::PartialSelect>
            + drizzle_core::query::RenderRelations<'q, MySQLValue<'q>>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        self.runner
            .query_rendered(common::render_relational_partial(self.builder))?
            .decode_relational_partial::<Table, Relations>()
    }
}

#[cfg(feature = "query")]
impl<'db, 'connection, 'q, Schema, Table, Relations, Where, Order>
    common::DrizzleQueryBuilder<
        'db,
        'q,
        &'db Transaction<'connection, Schema>,
        Schema,
        Table,
        Relations,
        drizzle_core::query::PartialColumns,
        drizzle_core::query::Clauses<Where, Order, drizzle_core::query::NoLimit>,
    >
{
    /// Executes the partial relational query with a one-row limit.
    ///
    /// # Errors
    ///
    /// Returns an error if execution or relational row decoding fails.
    pub fn find_first(
        self,
    ) -> Result<Option<<Relations as drizzle_core::query::BuildRow<Table::PartialSelect>>::Row>>
    where
        Table: drizzle_core::query::QueryTable,
        Table::PartialSelect: drizzle_core::query::FromJsonObject,
        Relations: drizzle_core::query::BuildRow<Table::PartialSelect>
            + drizzle_core::query::RenderRelations<'q, MySQLValue<'q>>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many()?.into_iter().next())
    }
}

/// A query attached to a blocking MySQL transaction.
pub type TransactionBuilder<'db, 'connection, Schema, Builder, State> =
    common::DrizzleBuilder<'db, &'db Transaction<'connection, Schema>, Schema, Builder, State>;

impl<'db, 'connection, 'q, Schema, State, Table, Marker, DecodedRow, Grouped>
    TransactionBuilder<
        'db,
        'connection,
        Schema,
        QueryBuilder<'q, Schema, State, Table, Marker, DecodedRow, Grouped>,
        State,
    >
where
    State: builder::ExecutableState,
{
    /// Executes this statement through MySQL's prepared/binary protocol.
    ///
    /// Returns affected-row and last-insert-ID metadata from the server's OK
    /// packet.
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction is unusable or execution fails.
    pub fn execute(self) -> Result<MySQLMutationResult> {
        self.runner.execute_rendered(self.builder)
    }

    /// Executes this query and decodes every row using its inferred marker.
    ///
    /// # Errors
    ///
    /// Returns an error if execution or row decoding fails.
    pub fn all<R, ScopeProof, AggProof>(self) -> Result<Vec<R>>
    where
        for<'row> Marker: DecodeSelectedRef<&'row MySQLRow<'row, Row>, R>
            + MarkerScopeValidFor<ScopeProof>
            + StrictDecodeMarker
            + MarkerColumnCountValid<MySQLRow<'row, Row>, DecodedRow, R>,
        Marker: MarkerAggValidFor<Grouped, AggProof>,
    {
        self.runner
            .query_rendered(self.builder)?
            .decode_all::<Marker, R>()
    }

    /// Executes this query and returns a decoded iterator over its
    /// materialized rows.
    ///
    /// # Errors
    ///
    /// Returns an error if execution or row decoding fails.
    pub fn rows(self) -> Result<Rows<DecodedRow>>
    where
        for<'row> DecodedRow: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        Ok(self
            .runner
            .query_rendered(self.builder)?
            .rows::<DecodedRow>())
    }

    /// Executes this query and decodes its first row.
    ///
    /// # Errors
    ///
    /// Returns an error if execution or decoding fails, or no row is returned.
    pub fn get<R, ScopeProof, AggProof>(self) -> Result<R>
    where
        for<'row> Marker: DecodeSelectedRef<&'row MySQLRow<'row, Row>, R>
            + MarkerScopeValidFor<ScopeProof>
            + StrictDecodeMarker
            + MarkerColumnCountValid<MySQLRow<'row, Row>, DecodedRow, R>,
        Marker: MarkerAggValidFor<Grouped, AggProof>,
    {
        self.runner
            .query_first_rendered(self.builder)?
            .decode_first::<Marker, R>()
    }

    /// Detaches a reusable prepared query from this transaction.
    #[must_use]
    pub fn prepare(
        self,
    ) -> crate::builder::mysql::mysql_sync::prepared::PreparedStatement<
        'q,
        Marker,
        DecodedRow,
        Grouped,
    > {
        crate::builder::mysql::mysql_sync::prepared::PreparedStatement::new(
            drizzle_core::prepared::prepare_render(&self.builder.into_sql()),
        )
    }
}

#[cfg(test)]
mod tests {
    use drizzle_mysql::{AccessMode as ConfigAccessMode, TransactionConfig};
    use mysql::{AccessMode, IsolationLevel};

    use super::{options, transaction_was_aborted};

    #[test]
    fn transaction_config_maps_to_mysql_driver_options() {
        let driver_options = options(
            TransactionConfig::builder()
                .repeatable_read()
                .read_only()
                .snapshot()
                .build(),
        );

        assert_eq!(
            driver_options.isolation_level(),
            Some(IsolationLevel::RepeatableRead)
        );
        assert_eq!(driver_options.access_mode(), Some(AccessMode::ReadOnly));
        assert!(driver_options.with_consistent_snapshot());

        let runtime = options(
            TransactionConfig::new()
                .isolation_level(drizzle_mysql::IsolationLevel::Serializable)
                .access_mode(ConfigAccessMode::ReadWrite),
        );
        assert_eq!(
            runtime.isolation_level(),
            Some(IsolationLevel::Serializable)
        );
        assert_eq!(runtime.access_mode(), Some(AccessMode::ReadWrite));
        assert!(!runtime.with_consistent_snapshot());
    }

    #[test]
    fn only_transaction_ending_errors_poison_the_wrapper() {
        let duplicate = mysql::Error::MySqlError(mysql::MySqlError {
            state: "23000".into(),
            message: "duplicate".into(),
            code: 1062,
        });
        let deadlock = mysql::Error::MySqlError(mysql::MySqlError {
            state: "40001".into(),
            message: "deadlock".into(),
            code: 1213,
        });
        let lock_timeout = mysql::Error::MySqlError(mysql::MySqlError {
            state: "HY000".into(),
            message: "lock wait timeout".into(),
            code: 1205,
        });

        assert!(!transaction_was_aborted(&duplicate));
        assert!(transaction_was_aborted(&deadlock));
        assert!(transaction_was_aborted(&lock_timeout));
    }
}
