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
use postgres::{
    Client, Row, Statement,
    types::{ToSql, Type},
};

use crate::builder::postgres::prepared_common::postgres_prepared_sync_impl;

const STATEMENT_CACHE_CAP: usize = 32;

static NEXT_CLIENT_ID: AtomicU64 = AtomicU64::new(1);
static REGISTERED_CLIENTS: Mutex<Vec<ClientRegistration>> = Mutex::new(Vec::new());

#[derive(Clone, Copy)]
struct ClientRegistration {
    client_key: usize,
    client_id: u64,
}

pub(crate) fn next_client_id() -> u64 {
    NEXT_CLIENT_ID.fetch_add(1, Ordering::Relaxed)
}

pub(crate) fn register_client(client: &Client, client_id: u64) {
    let client_key = client as *const Client as usize;
    let mut registrations = REGISTERED_CLIENTS
        .lock()
        .unwrap_or_else(|err| err.into_inner());

    registrations.retain(|entry| entry.client_id == client_id || entry.client_key != client_key);
    if let Some(entry) = registrations
        .iter_mut()
        .find(|entry| entry.client_id == client_id)
    {
        entry.client_key = client_key;
    } else {
        registrations.push(ClientRegistration {
            client_key,
            client_id,
        });
    }
}

pub(crate) fn unregister_client(client_id: u64) {
    let mut registrations = REGISTERED_CLIENTS
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    registrations.retain(|entry| entry.client_id != client_id);
}

fn registered_client_id(client: &Client) -> Option<u64> {
    let client_key = client as *const Client as usize;
    let registrations = REGISTERED_CLIENTS
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    registrations
        .iter()
        .find(|entry| entry.client_key == client_key)
        .map(|entry| entry.client_id)
}

/// A prepared statement that can be executed multiple times with different parameters.
///
/// This statement can be run against a `postgres` client.
#[derive(Debug, Clone)]
pub struct PreparedStatement<'a, Marker = (), DecodedRow = ()> {
    pub(crate) inner: CorePreparedStatement<'a, PostgresValue<'a>>,
    pub(crate) statement_cache: StatementCache,
    pub(crate) marker: PhantomData<(Marker, DecodedRow)>,
}

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
    pub(crate) fn statement(
        &self,
        client: &mut Client,
        sql: &str,
        param_types: &[Type],
    ) -> Result<Statement, postgres::Error> {
        let Some(client_id) = registered_client_id(client) else {
            return client.prepare_typed(sql, param_types);
        };

        {
            let mut cache = self.0.lock().unwrap_or_else(|err| err.into_inner());
            if let Some(pos) = cache.iter().position(|cached| {
                cached.client_id == client_id
                    && cached.sql.as_ref() == sql
                    && cached.param_types.as_ref() == param_types
            }) {
                let cached = cache.remove(pos);
                let statement = cached.statement.clone();
                cache.insert(0, cached);
                return Ok(statement);
            }
        }

        let statement = client.prepare_typed(sql, param_types)?;
        let mut cache = self.0.lock().unwrap_or_else(|err| err.into_inner());
        if let Some(pos) = cache.iter().position(|cached| {
            cached.client_id == client_id
                && cached.sql.as_ref() == sql
                && cached.param_types.as_ref() == param_types
        }) {
            let cached = cache.remove(pos);
            let statement = cached.statement.clone();
            cache.insert(0, cached);
            return Ok(statement);
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
        Ok(statement)
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

    pub(crate) fn driver_statement(
        &self,
        client: &mut Client,
        sql: &str,
        param_types: &[Type],
    ) -> Result<Statement, postgres::Error> {
        self.statement_cache.statement(client, sql, param_types)
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

    pub(crate) fn driver_statement(
        &self,
        client: &mut Client,
        sql: &str,
        param_types: &[Type],
    ) -> Result<Statement, postgres::Error> {
        self.statement_cache.statement(client, sql, param_types)
    }
}

postgres_prepared_sync_impl!(Client, Row, ToSql);

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
