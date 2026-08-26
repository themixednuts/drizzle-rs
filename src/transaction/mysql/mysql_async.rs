//! Async MySQL transactions backed by `mysql_async`.

use core::sync::atomic::{AtomicBool, Ordering};

use drizzle_core::{
    error::{DrizzleError, QueryContext, Result, ResultExt},
    row::{
        DecodeSelectedRef, FromDrizzleRow, MarkerAggValidFor, MarkerColumnCountValid,
        MarkerScopeValidFor, StrictDecodeMarker,
    },
    traits::ToSQL,
};
use drizzle_mysql::{
    MySQLAccessMode, MySQLIsolationLevel, MySQLMutationResult, MySQLRow, MySQLTransactionConfig,
    builder::{
        self, DeleteBuilder, DeleteInitial, InsertBuilder, InsertInitial, QueryBuilder,
        SelectBuilder, SelectInitial, UpdateBuilder, UpdateInitial,
    },
    traits::MySQLTable,
    values::MySQLValue,
};
use mysql_async::{
    IsolationLevel, Row, Transaction as DriverTransaction, TxOpts, prelude::Queryable,
};

use crate::{
    builder::mysql::{
        common::{self, DrizzleBuilder},
        driver_common::{QueryOutput, render},
        mysql_async::{
            driver_error, execute_request_observing, initialize_session_observing,
            query_first_request_observing, query_request_observing,
        },
    },
    transaction::savepoint::{AsyncSavepointState, async_savepoint},
};

fn consumed() -> DrizzleError {
    DrizzleError::TransactionError("MySQL transaction already consumed".into())
}

fn aborted() -> DrizzleError {
    DrizzleError::TransactionError(
        "MySQL transaction is unusable after the server aborted it".into(),
    )
}

fn transaction_was_aborted(error: &mysql_async::Error) -> bool {
    error.is_fatal()
        || matches!(error, mysql_async::Error::Server(error) if error.code == 1205 || error.state.starts_with("40"))
}

pub(crate) fn options(config: MySQLTransactionConfig) -> TxOpts {
    let mut options = TxOpts::default();
    if let Some(isolation) = config.isolation() {
        options.with_isolation_level(match isolation {
            MySQLIsolationLevel::ReadUncommitted => IsolationLevel::ReadUncommitted,
            MySQLIsolationLevel::ReadCommitted => IsolationLevel::ReadCommitted,
            MySQLIsolationLevel::RepeatableRead => IsolationLevel::RepeatableRead,
            MySQLIsolationLevel::Serializable => IsolationLevel::Serializable,
        });
    }
    if let Some(access) = config.access() {
        options.with_readonly(matches!(access, MySQLAccessMode::ReadOnly));
    }
    options.with_consistent_snapshot(config.consistent_snapshot());
    options
}

/// A scoped async MySQL transaction.
///
/// Explicit [`commit`](Self::commit) and [`rollback`](Self::rollback) await the
/// server response. Dropping an active transaction preserves
/// `mysql_async`'s delayed rollback contract: a direct connection cleans it
/// before its next command, while a pooled connection is cleaned by the pool
/// recycler before reuse.
pub struct Transaction<'connection, Schema = ()> {
    transaction: tokio::sync::Mutex<Option<DriverTransaction<'connection>>>,
    schema: Schema,
    savepoints: AsyncSavepointState,
    poisoned: AtomicBool,
    session_ready: AtomicBool,
}

impl<Schema> core::fmt::Debug for Transaction<'_, Schema> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("Transaction")
            .field(
                "active",
                &self
                    .transaction
                    .try_lock()
                    .map(|transaction| transaction.is_some())
                    .unwrap_or(true),
            )
            .field("poisoned", &self.poisoned.load(Ordering::Acquire))
            .field("savepoints", &self.savepoints)
            .finish_non_exhaustive()
    }
}

impl<'connection, Schema> Transaction<'connection, Schema> {
    pub(crate) fn new(
        transaction: DriverTransaction<'connection>,
        schema: Schema,
        session_ready: bool,
    ) -> Self {
        Self {
            transaction: tokio::sync::Mutex::new(Some(transaction)),
            schema,
            savepoints: AsyncSavepointState::new(),
            poisoned: AtomicBool::new(false),
            session_ready: AtomicBool::new(session_ready),
        }
    }

    fn ensure_usable(&self) -> Result<()> {
        self.savepoints.ensure_usable()?;
        if self.poisoned.load(Ordering::Acquire) {
            Err(aborted())
        } else {
            Ok(())
        }
    }

    fn observe_error(&self, error: &mysql_async::Error) {
        if transaction_was_aborted(error) {
            self.poisoned.store(true, Ordering::Release);
        }
    }

    async fn ensure_session(&self) -> Result<()> {
        self.ensure_usable()?;
        if self.session_ready.load(Ordering::Acquire) {
            return Ok(());
        }

        let mut transaction = self.transaction.lock().await;
        initialize_session_observing(transaction.as_mut().ok_or_else(consumed)?, |error| {
            self.observe_error(error);
        })
        .await?;
        self.session_ready.store(true, Ordering::Release);
        Ok(())
    }

    pub(crate) async fn initialize(&self) -> Result<()> {
        self.ensure_session().await
    }

    #[must_use]
    pub const fn schema(&self) -> &Schema {
        &self.schema
    }

    pub(crate) async fn execute_rendered<'q>(
        &self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<MySQLMutationResult> {
        self.ensure_session().await?;
        let (sql, values) = render(query);
        let mut transaction = self.transaction.lock().await;
        execute_request_observing(
            transaction.as_mut().ok_or_else(consumed)?,
            &sql,
            &values,
            |error| self.observe_error(error),
        )
        .await
    }

    pub(crate) async fn query_rendered<'q>(
        &self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<QueryOutput<'q>> {
        self.ensure_session().await?;
        let (sql, values) = render(query);
        let mut transaction = self.transaction.lock().await;
        let rows = query_request_observing(
            transaction.as_mut().ok_or_else(consumed)?,
            &sql,
            &values,
            |error| self.observe_error(error),
        )
        .await?;
        Ok(QueryOutput::new(sql, values, rows))
    }

    pub(crate) async fn query_first_rendered<'q>(
        &self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<QueryOutput<'q>> {
        self.ensure_session().await?;
        let (sql, values) = render(query);
        let mut transaction = self.transaction.lock().await;
        let rows = query_first_request_observing(
            transaction.as_mut().ok_or_else(consumed)?,
            &sql,
            &values,
            |error| self.observe_error(error),
        )
        .await?
        .into_iter()
        .collect();
        Ok(QueryOutput::new(sql, values, rows))
    }

    async fn execute_raw(&self, sql: &str) -> Result<()> {
        self.ensure_usable()?;
        drizzle_core::drizzle_trace_query!(sql, 0);
        let mut transaction = self.transaction.lock().await;
        transaction
            .as_mut()
            .ok_or_else(consumed)?
            .query_drop(sql)
            .await
            .map_err(|error| {
                self.poisoned.store(true, Ordering::Release);
                driver_error(error)
            })
            .with_query(|| QueryContext::new::<MySQLValue<'_>>(sql, &[]))
    }

    pub async fn commit(self) -> Result<()> {
        let unusable = self.ensure_usable().err();
        let transaction = self.transaction.into_inner().ok_or_else(consumed)?;
        if let Some(reason) = unusable {
            return match transaction.rollback().await {
                Ok(()) => Err(reason),
                Err(error) => Err(DrizzleError::TransactionError(
                    format!("{reason}; rollback failed: {error}").into(),
                )),
            };
        }
        transaction.commit().await.map_err(driver_error)
    }

    pub async fn rollback(self) -> Result<()> {
        self.transaction
            .into_inner()
            .ok_or_else(consumed)?
            .rollback()
            .await
            .map_err(driver_error)
    }

    pub async fn savepoint<F, R>(&self, body: F) -> Result<R>
    where
        F: AsyncFnOnce(&Self) -> Result<R>,
    {
        self.ensure_usable()?;
        async_savepoint(
            &self.savepoints,
            |sql| async move { self.execute_raw(&sql).await },
            body(self),
        )
        .await
    }

    pub async fn execute<'q>(
        &self,
        query: impl ToSQL<'q, MySQLValue<'q>>,
    ) -> Result<MySQLMutationResult> {
        let result = self.execute_rendered(query).await;
        self.session_ready.store(false, Ordering::Release);
        result
    }

    pub async fn all<'q, R>(&self, query: impl ToSQL<'q, MySQLValue<'q>>) -> Result<Vec<R>>
    where
        for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        self.query_rendered(query).await?.decode_all_rows()
    }

    pub async fn get<'q, R>(&self, query: impl ToSQL<'q, MySQLValue<'q>>) -> Result<R>
    where
        for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        self.query_first_rendered(query).await?.decode_first_row()
    }

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

    mysql_shared_builder_constructors!(&'db Transaction<'connection, Schema>);
}

#[cfg(feature = "query")]
impl<Schema> common::RelationalPreparedDriver for &Transaction<'_, Schema> {
    type PreparedDriver = crate::builder::mysql::mysql_async::RelationalPrepared;
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
    pub async fn find_many(
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
            .query_rendered(common::render_relational_all(self.builder))
            .await?
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
    pub async fn find_first(
        self,
    ) -> Result<Option<<Relations as drizzle_core::query::BuildRow<Table::Select>>::Row>>
    where
        Table: drizzle_core::query::QueryTable,
        for<'row> Table::Select: FromDrizzleRow<MySQLRow<'row, Row>>,
        Relations: drizzle_core::query::BuildRow<Table::Select>
            + drizzle_core::query::RenderRelations<'q, MySQLValue<'q>>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many().await?.into_iter().next())
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
    pub async fn find_many(
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
            .query_rendered(common::render_relational_partial(self.builder))
            .await?
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
    pub async fn find_first(
        self,
    ) -> Result<Option<<Relations as drizzle_core::query::BuildRow<Table::PartialSelect>>::Row>>
    where
        Table: drizzle_core::query::QueryTable,
        Table::PartialSelect: drizzle_core::query::FromJsonObject,
        Relations: drizzle_core::query::BuildRow<Table::PartialSelect>
            + drizzle_core::query::RenderRelations<'q, MySQLValue<'q>>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        Ok(self.limit(1).find_many().await?.into_iter().next())
    }
}

/// A query attached to an async MySQL transaction.
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
    pub async fn execute(self) -> Result<MySQLMutationResult> {
        self.runner.execute_rendered(self.builder).await
    }

    pub async fn all<R, ScopeProof, AggProof>(self) -> Result<Vec<R>>
    where
        for<'row> Marker: DecodeSelectedRef<&'row MySQLRow<'row, Row>, R>
            + MarkerScopeValidFor<ScopeProof>
            + StrictDecodeMarker
            + MarkerColumnCountValid<MySQLRow<'row, Row>, DecodedRow, R>,
        Marker: MarkerAggValidFor<Grouped, AggProof>,
    {
        self.runner
            .query_rendered(self.builder)
            .await?
            .decode_all::<Marker, R>()
    }

    pub async fn get<R, ScopeProof, AggProof>(self) -> Result<R>
    where
        for<'row> Marker: DecodeSelectedRef<&'row MySQLRow<'row, Row>, R>
            + MarkerScopeValidFor<ScopeProof>
            + StrictDecodeMarker
            + MarkerColumnCountValid<MySQLRow<'row, Row>, DecodedRow, R>,
        Marker: MarkerAggValidFor<Grouped, AggProof>,
    {
        self.runner
            .query_first_rendered(self.builder)
            .await?
            .decode_first::<Marker, R>()
    }

    #[must_use]
    pub fn prepare(
        self,
    ) -> crate::builder::mysql::mysql_async::prepared::PreparedStatement<
        'q,
        Marker,
        DecodedRow,
        Grouped,
    > {
        crate::builder::mysql::mysql_async::prepared::PreparedStatement::new(
            drizzle_core::prepared::prepare_render(&self.builder.into_sql()),
        )
    }
}

#[cfg(test)]
mod tests {
    use drizzle_mysql::{MySQLAccessMode, MySQLIsolationLevel, MySQLTransactionConfig};
    use mysql_async::IsolationLevel;

    use super::{options, transaction_was_aborted};

    #[test]
    fn transaction_config_maps_to_mysql_driver_options() {
        let options = options(
            MySQLTransactionConfig::default()
                .isolation_level(MySQLIsolationLevel::Serializable)
                .access_mode(MySQLAccessMode::ReadOnly)
                .with_consistent_snapshot(),
        );

        assert_eq!(
            options.isolation_level(),
            Some(IsolationLevel::Serializable)
        );
        assert_eq!(options.readonly(), Some(true));
        assert!(options.consistent_snapshot());
    }

    #[test]
    fn only_transaction_ending_errors_poison_the_wrapper() {
        let duplicate = mysql_async::Error::Server(mysql_async::ServerError {
            state: "23000".into(),
            message: "duplicate".into(),
            code: 1062,
        });
        let deadlock = mysql_async::Error::Server(mysql_async::ServerError {
            state: "40001".into(),
            message: "deadlock".into(),
            code: 1213,
        });
        let lock_timeout = mysql_async::Error::Server(mysql_async::ServerError {
            state: "HY000".into(),
            message: "lock wait timeout".into(),
            code: 1205,
        });

        assert!(!transaction_was_aborted(&duplicate));
        assert!(transaction_was_aborted(&deadlock));
        assert!(transaction_was_aborted(&lock_timeout));
    }
}
