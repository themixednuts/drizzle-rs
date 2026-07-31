use std::{
    borrow::Cow,
    marker::PhantomData,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use drizzle_core::{
    param::{OwnedParam, Param},
    prepared::{
        OwnedPreparedStatement as CoreOwnedPreparedStatement,
        PreparedStatement as CorePreparedStatement,
    },
    traits::ToSQL,
};
use drizzle_postgres::values::{OwnedPostgresValue, PostgresValue};
use tokio_postgres::{
    Client, Row, Statement,
    types::{ToSql, Type},
};

use crate::builder::postgres::prepared_common::postgres_prepared_async_impl;

/// A prepared statement that can be executed multiple times with different parameters.
///
/// This statement can be run against a `tokio-postgres` client.
#[derive(Debug, Clone)]
pub struct PreparedStatement<'a, Marker = (), DecodedRow = ()> {
    pub(crate) inner: CorePreparedStatement<'a, PostgresValue<'a>>,
    pub(crate) statement_cache: StatementCache,
    pub(crate) marker: PhantomData<(Marker, DecodedRow)>,
}

const STATEMENT_CACHE_CAP: usize = 32;

static NEXT_CLIENT_ID: AtomicU64 = AtomicU64::new(1);
static REGISTERED_CLIENTS: Mutex<Vec<ClientEntry>> = Mutex::new(Vec::new());

#[derive(Clone, Copy)]
struct ClientEntry {
    client_key: usize,
    client_id: u64,
}

fn register_client(client: &Client, client_id: u64) {
    let client_key = std::ptr::from_ref(client) as usize;
    let mut registrations = REGISTERED_CLIENTS
        .lock()
        .unwrap_or_else(|err| err.into_inner());

    // A reused address belongs to the newest client; drop stale entries so
    // lookups never resolve an old id for it.
    registrations.retain(|entry| entry.client_id == client_id || entry.client_key != client_key);
    if let Some(entry) = registrations
        .iter_mut()
        .find(|entry| entry.client_id == client_id)
    {
        entry.client_key = client_key;
    } else {
        registrations.push(ClientEntry {
            client_key,
            client_id,
        });
    }
}

fn unregister_client(client_id: u64) {
    let mut registrations = REGISTERED_CLIENTS
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    registrations.retain(|entry| entry.client_id != client_id);
}

fn registered_client_id(client: &Client) -> Option<u64> {
    let client_key = std::ptr::from_ref(client) as usize;
    let registrations = REGISTERED_CLIENTS
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    registrations
        .iter()
        .find(|entry| entry.client_key == client_key)
        .map(|entry| entry.client_id)
}

/// Registers a `Drizzle`-owned client under a fresh identity for the
/// lifetime of the guard.
///
/// The client lives behind an `Arc` inside `Drizzle`, so its address is
/// stable for as long as any clone exists; the guard (shared via `Arc`
/// alongside the client) unregisters on the last drop.
pub(crate) struct ClientRegistration {
    client_id: u64,
}

impl ClientRegistration {
    pub(crate) fn new(client: &Client) -> Self {
        let client_id = NEXT_CLIENT_ID.fetch_add(1, Ordering::Relaxed);
        register_client(client, client_id);
        Self { client_id }
    }
}

impl Drop for ClientRegistration {
    fn drop(&mut self) {
        unregister_client(self.client_id);
    }
}

impl std::fmt::Debug for ClientRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientRegistration").finish_non_exhaustive()
    }
}

/// Statement cache for the public `&Client` execution API.
///
/// `tokio-postgres` statements are connection-bound, but a borrowed `Client`
/// address alone is not a sound cache key — the address can be reused after
/// the connection is dropped. Caching therefore only applies to clients
/// registered through [`ClientRegistration`] (i.e. owned by a `Drizzle`),
/// keyed by their registration id; unregistered clients prepare per call.
#[derive(Clone, Default)]
pub(crate) struct StatementCache(Arc<Mutex<Vec<CachedStatement>>>);

struct CachedStatement {
    client_id: u64,
    sql: Box<str>,
    param_types: Box<[Type]>,
    statement: Statement,
}

impl std::fmt::Debug for StatementCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatementCache").finish_non_exhaustive()
    }
}

impl StatementCache {
    pub(crate) async fn statement(
        &self,
        client: &Client,
        sql: &str,
        param_types: &[Type],
    ) -> Result<Statement, tokio_postgres::Error> {
        let Some(client_id) = registered_client_id(client) else {
            return client.prepare_typed(sql, param_types).await;
        };

        if let Some(statement) = self.lookup(client_id, sql, param_types) {
            return Ok(statement);
        }
        let statement = client.prepare_typed(sql, param_types).await?;
        self.store(client_id, sql, param_types, &statement);
        Ok(statement)
    }

    fn lookup(&self, client_id: u64, sql: &str, param_types: &[Type]) -> Option<Statement> {
        let mut cache = self.0.lock().unwrap_or_else(|err| err.into_inner());
        let pos = cache.iter().position(|cached| {
            cached.client_id == client_id
                && cached.sql.as_ref() == sql
                && cached.param_types.as_ref() == param_types
        })?;
        let cached = cache.remove(pos);
        let statement = cached.statement.clone();
        cache.insert(0, cached);
        Some(statement)
    }

    fn store(&self, client_id: u64, sql: &str, param_types: &[Type], statement: &Statement) {
        let mut cache = self.0.lock().unwrap_or_else(|err| err.into_inner());
        if cache.iter().any(|cached| {
            cached.client_id == client_id
                && cached.sql.as_ref() == sql
                && cached.param_types.as_ref() == param_types
        }) {
            return;
        }
        cache.insert(
            0,
            CachedStatement {
                client_id,
                sql: sql.into(),
                param_types: param_types.into(),
                statement: statement.clone(),
            },
        );
        cache.truncate(STATEMENT_CACHE_CAP);
    }
}

/// LRU statement cache for the `Drizzle`-owned execution paths.
///
/// Unlike [`StatementCache`], this cache never needs a connection identity:
/// it lives inside `Drizzle` next to the `Arc<Client>` it serves, so every
/// lookup and insert targets the same connection for the cache's whole
/// lifetime (clones of `Drizzle` share both the client and this cache).
#[derive(Clone, Default)]
pub(crate) struct ClientStatementCache(Arc<Mutex<Vec<ClientCachedStatement>>>);

struct ClientCachedStatement {
    sql: Box<str>,
    param_types: Box<[Type]>,
    statement: Statement,
}

impl std::fmt::Debug for ClientStatementCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientStatementCache")
            .finish_non_exhaustive()
    }
}

impl ClientStatementCache {
    pub(crate) async fn statement(
        &self,
        client: &Client,
        sql: &str,
        param_types: &[Type],
    ) -> Result<Statement, tokio_postgres::Error> {
        if let Some(statement) = self.lookup(sql, param_types) {
            return Ok(statement);
        }
        // Concurrent misses may prepare the same statement twice; the loser's
        // copy is simply dropped (queuing a close), which is harmless.
        let statement = client.prepare_typed(sql, param_types).await?;
        self.store(sql, param_types, &statement);
        Ok(statement)
    }

    fn lookup(&self, sql: &str, param_types: &[Type]) -> Option<Statement> {
        let mut cache = self.0.lock().unwrap_or_else(|err| err.into_inner());
        let pos = cache.iter().position(|cached| {
            cached.sql.as_ref() == sql && cached.param_types.as_ref() == param_types
        })?;
        let cached = cache.remove(pos);
        let statement = cached.statement.clone();
        cache.insert(0, cached);
        Some(statement)
    }

    fn store(&self, sql: &str, param_types: &[Type], statement: &Statement) {
        let mut cache = self.0.lock().unwrap_or_else(|err| err.into_inner());
        if cache
            .iter()
            .any(|cached| cached.sql.as_ref() == sql && cached.param_types.as_ref() == param_types)
        {
            return;
        }
        cache.insert(
            0,
            ClientCachedStatement {
                sql: sql.into(),
                param_types: param_types.into(),
                statement: statement.clone(),
            },
        );
        cache.truncate(STATEMENT_CACHE_CAP);
    }
}

impl<Marker, DecodedRow> From<OwnedPreparedStatement<Marker, DecodedRow>>
    for PreparedStatement<'_, Marker, DecodedRow>
{
    fn from(value: OwnedPreparedStatement<Marker, DecodedRow>) -> Self {
        let postgres_params = value.inner.params.iter().map(|v| {
            Param::new(
                v.placeholder,
                v.value.clone().map(|v| Cow::Owned(PostgresValue::from(v))),
            )
        });
        let inner = CorePreparedStatement {
            text_segments: value.inner.text_segments,
            params: postgres_params.collect::<Box<[_]>>(),
            sql: value.inner.sql,
        };
        PreparedStatement {
            inner,
            statement_cache: value.statement_cache,
            marker: PhantomData,
        }
    }
}

impl<'a, Marker, DecodedRow> PreparedStatement<'a, Marker, DecodedRow> {
    pub(crate) fn new(inner: CorePreparedStatement<'a, PostgresValue<'a>>) -> Self {
        Self {
            inner,
            statement_cache: StatementCache::default(),
            marker: PhantomData,
        }
    }

    /// Gets the SQL query string with placeholders
    pub fn sql(&self) -> &str {
        self.inner.sql()
    }

    pub(crate) async fn driver_statement(
        &self,
        client: &Client,
        sql: &str,
        param_types: &[Type],
    ) -> Result<Statement, tokio_postgres::Error> {
        self.statement_cache
            .statement(client, sql, param_types)
            .await
    }

    /// Gets the number of parameters in the query
    pub fn param_count(&self) -> usize {
        self.inner.params.len()
    }

    /// Converts this borrowed prepared statement into an owned one.
    pub fn into_owned(self) -> OwnedPreparedStatement<Marker, DecodedRow> {
        let owned_params = self
            .inner
            .params
            .into_vec()
            .into_iter()
            .map(|p| OwnedParam {
                placeholder: p.placeholder,
                value: p.value.map(|v| OwnedPostgresValue::from(v.into_owned())),
            });

        let inner = CoreOwnedPreparedStatement {
            text_segments: self.inner.text_segments,
            params: owned_params.collect::<Box<[_]>>(),
            sql: self.inner.sql,
        };

        OwnedPreparedStatement {
            inner,
            statement_cache: self.statement_cache,
            marker: PhantomData,
        }
    }
}

/// Owned `PostgreSQL` prepared statement wrapper.
///
/// This is the owned counterpart to [`PreparedStatement`] that doesn't have any lifetime
/// constraints.
#[derive(Debug, Clone)]
pub struct OwnedPreparedStatement<Marker = (), DecodedRow = ()> {
    pub(crate) inner: CoreOwnedPreparedStatement<OwnedPostgresValue>,
    pub(crate) statement_cache: StatementCache,
    pub(crate) marker: PhantomData<(Marker, DecodedRow)>,
}

impl<'a, Marker, DecodedRow> From<PreparedStatement<'a, Marker, DecodedRow>>
    for OwnedPreparedStatement<Marker, DecodedRow>
{
    fn from(value: PreparedStatement<'a, Marker, DecodedRow>) -> Self {
        value.into_owned()
    }
}

impl<Marker, DecodedRow> OwnedPreparedStatement<Marker, DecodedRow> {
    /// Gets the SQL query string with placeholders
    pub fn sql(&self) -> &str {
        self.inner.sql()
    }

    /// Gets the number of parameters in the query
    pub fn param_count(&self) -> usize {
        self.inner.params.len()
    }

    pub(crate) async fn driver_statement(
        &self,
        client: &Client,
        sql: &str,
        param_types: &[Type],
    ) -> Result<Statement, tokio_postgres::Error> {
        self.statement_cache
            .statement(client, sql, param_types)
            .await
    }
}

postgres_prepared_async_impl!(Client, Row, ToSql);

impl<Marker, DecodedRow> std::fmt::Display for PreparedStatement<'_, Marker, DecodedRow> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl<Marker, DecodedRow> std::fmt::Display for OwnedPreparedStatement<Marker, DecodedRow> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl<'a, Marker, DecodedRow> ToSQL<'a, PostgresValue<'a>>
    for PreparedStatement<'a, Marker, DecodedRow>
{
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, PostgresValue<'a>> {
        self.inner.to_sql()
    }
}

impl<'a, Marker, DecodedRow> ToSQL<'a, OwnedPostgresValue>
    for OwnedPreparedStatement<Marker, DecodedRow>
{
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, OwnedPostgresValue> {
        self.inner.to_sql()
    }
}

impl<'a, Marker, DecodedRow> ToSQL<'a, PostgresValue<'a>>
    for OwnedPreparedStatement<Marker, DecodedRow>
{
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, PostgresValue<'a>> {
        self.inner.to_sql().map_params(PostgresValue::from)
    }
}
