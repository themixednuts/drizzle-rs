//! Detached prepared statements for the Tokio MySQL adapter.

use core::marker::PhantomData;
use std::borrow::Cow;

use drizzle_core::{
    error::Result,
    param::{OwnedParam, Param, ParamBind},
    prepared::{
        OwnedPreparedStatement as CoreOwnedPreparedStatement,
        PreparedStatement as CorePreparedStatement,
    },
    row::{
        DecodeSelectedRef, MarkerAggValidFor, MarkerColumnCountValid, MarkerScopeValidFor,
        StrictDecodeMarker,
    },
    traits::ToSQL,
};
use drizzle_mysql::{
    MySQLMutationResult, MySQLRow,
    values::{MySQLValue, OwnedMySQLValue},
};
use mysql_async::{Row, prelude::ToConnection};

use crate::builder::mysql::driver_common::QueryOutput;

use super::{execute_request, initialize_session, query_first_request, query_request};

/// A reusable prepared MySQL statement borrowing its SQL values.
#[derive(Debug, Clone)]
pub struct PreparedStatement<'q, Marker = (), DecodedRow = (), Grouped = ()> {
    pub(crate) inner: CorePreparedStatement<'q, MySQLValue<'q>>,
    marker: PhantomData<(Marker, DecodedRow, Grouped)>,
}

impl<'q, Marker, DecodedRow, Grouped> PreparedStatement<'q, Marker, DecodedRow, Grouped> {
    pub(crate) const fn new(inner: CorePreparedStatement<'q, MySQLValue<'q>>) -> Self {
        Self {
            inner,
            marker: PhantomData,
        }
    }

    #[must_use]
    pub fn sql(&self) -> &str {
        self.inner.sql()
    }

    #[must_use]
    pub fn param_count(&self) -> usize {
        self.inner.external_param_count()
    }

    pub async fn execute<'connection, 'transaction, Connection>(
        &self,
        connection: Connection,
        params: impl IntoIterator<Item = ParamBind<'q, MySQLValue<'q>>>,
    ) -> Result<MySQLMutationResult>
    where
        'transaction: 'connection,
        Connection: ToConnection<'connection, 'transaction>,
    {
        let mut connection = connection
            .to_connection()
            .resolve()
            .await
            .map_err(super::driver_error)?;
        initialize_session(&mut connection).await?;
        let (sql, values) = self.inner.bind(params)?;
        execute_request(&mut connection, sql, &values.collect::<Vec<_>>()).await
    }

    pub async fn all<'connection, 'transaction, Connection, R, ScopeProof, AggProof>(
        &self,
        connection: Connection,
        params: impl IntoIterator<Item = ParamBind<'q, MySQLValue<'q>>>,
    ) -> Result<Vec<R>>
    where
        'transaction: 'connection,
        Connection: ToConnection<'connection, 'transaction>,
        for<'row> Marker: DecodeSelectedRef<&'row MySQLRow<'row, Row>, R>
            + MarkerScopeValidFor<ScopeProof>
            + StrictDecodeMarker
            + MarkerColumnCountValid<MySQLRow<'row, Row>, DecodedRow, R>,
        Marker: MarkerAggValidFor<Grouped, AggProof>,
    {
        let mut connection = connection
            .to_connection()
            .resolve()
            .await
            .map_err(super::driver_error)?;
        initialize_session(&mut connection).await?;
        let (sql, values) = self.inner.bind(params)?;
        let values = values.collect::<Vec<_>>();
        let rows = query_request(&mut connection, sql, &values).await?;
        QueryOutput::new(sql.to_owned(), values, rows).decode_all::<Marker, R>()
    }

    pub async fn get<'connection, 'transaction, Connection, R, ScopeProof, AggProof>(
        &self,
        connection: Connection,
        params: impl IntoIterator<Item = ParamBind<'q, MySQLValue<'q>>>,
    ) -> Result<R>
    where
        'transaction: 'connection,
        Connection: ToConnection<'connection, 'transaction>,
        for<'row> Marker: DecodeSelectedRef<&'row MySQLRow<'row, Row>, R>
            + MarkerScopeValidFor<ScopeProof>
            + StrictDecodeMarker
            + MarkerColumnCountValid<MySQLRow<'row, Row>, DecodedRow, R>,
        Marker: MarkerAggValidFor<Grouped, AggProof>,
    {
        let mut connection = connection
            .to_connection()
            .resolve()
            .await
            .map_err(super::driver_error)?;
        initialize_session(&mut connection).await?;
        let (sql, values) = self.inner.bind(params)?;
        let values = values.collect::<Vec<_>>();
        let rows = query_first_request(&mut connection, sql, &values)
            .await?
            .into_iter()
            .collect();
        QueryOutput::new(sql.to_owned(), values, rows).decode_first::<Marker, R>()
    }

    #[must_use]
    pub fn into_owned(self) -> OwnedPreparedStatement<Marker, DecodedRow, Grouped> {
        let params = self
            .inner
            .params
            .into_vec()
            .into_iter()
            .map(|param| OwnedParam {
                placeholder: param.placeholder,
                value: param
                    .value
                    .map(|value| OwnedMySQLValue::from(value.into_owned())),
            });
        OwnedPreparedStatement {
            inner: CoreOwnedPreparedStatement {
                text_segments: self.inner.text_segments,
                params: params.collect(),
                sql: self.inner.sql,
            },
            marker: PhantomData,
        }
    }
}

/// Owned counterpart to [`PreparedStatement`].
#[derive(Debug, Clone)]
pub struct OwnedPreparedStatement<Marker = (), DecodedRow = (), Grouped = ()> {
    inner: CoreOwnedPreparedStatement<OwnedMySQLValue>,
    marker: PhantomData<(Marker, DecodedRow, Grouped)>,
}

impl<Marker, DecodedRow, Grouped> OwnedPreparedStatement<Marker, DecodedRow, Grouped> {
    fn borrowed(&self) -> PreparedStatement<'_, Marker, DecodedRow, Grouped> {
        let params = self.inner.params.iter().map(|param| {
            Param::new(
                param.placeholder,
                param
                    .value
                    .clone()
                    .map(|value| Cow::Owned(MySQLValue::from(value))),
            )
        });
        PreparedStatement::new(CorePreparedStatement {
            text_segments: self.inner.text_segments.clone(),
            params: params.collect(),
            sql: self.inner.sql.clone(),
        })
    }

    #[must_use]
    pub fn sql(&self) -> &str {
        self.inner.sql()
    }

    #[must_use]
    pub fn param_count(&self) -> usize {
        self.inner.external_param_count()
    }

    pub async fn execute<'a, 'connection, 'transaction, Connection>(
        &'a self,
        connection: Connection,
        params: impl IntoIterator<Item = ParamBind<'a, MySQLValue<'a>>>,
    ) -> Result<MySQLMutationResult>
    where
        'transaction: 'connection,
        Connection: ToConnection<'connection, 'transaction>,
    {
        self.borrowed().execute(connection, params).await
    }

    pub async fn all<'a, 'connection, 'transaction, Connection, R, ScopeProof, AggProof>(
        &'a self,
        connection: Connection,
        params: impl IntoIterator<Item = ParamBind<'a, MySQLValue<'a>>>,
    ) -> Result<Vec<R>>
    where
        'transaction: 'connection,
        Connection: ToConnection<'connection, 'transaction>,
        for<'row> Marker: DecodeSelectedRef<&'row MySQLRow<'row, Row>, R>
            + MarkerScopeValidFor<ScopeProof>
            + StrictDecodeMarker
            + MarkerColumnCountValid<MySQLRow<'row, Row>, DecodedRow, R>,
        Marker: MarkerAggValidFor<Grouped, AggProof>,
    {
        self.borrowed().all(connection, params).await
    }

    pub async fn get<'a, 'connection, 'transaction, Connection, R, ScopeProof, AggProof>(
        &'a self,
        connection: Connection,
        params: impl IntoIterator<Item = ParamBind<'a, MySQLValue<'a>>>,
    ) -> Result<R>
    where
        'transaction: 'connection,
        Connection: ToConnection<'connection, 'transaction>,
        for<'row> Marker: DecodeSelectedRef<&'row MySQLRow<'row, Row>, R>
            + MarkerScopeValidFor<ScopeProof>
            + StrictDecodeMarker
            + MarkerColumnCountValid<MySQLRow<'row, Row>, DecodedRow, R>,
        Marker: MarkerAggValidFor<Grouped, AggProof>,
    {
        self.borrowed().get(connection, params).await
    }
}

impl<Marker, DecodedRow, Grouped> From<PreparedStatement<'_, Marker, DecodedRow, Grouped>>
    for OwnedPreparedStatement<Marker, DecodedRow, Grouped>
{
    fn from(value: PreparedStatement<'_, Marker, DecodedRow, Grouped>) -> Self {
        value.into_owned()
    }
}

impl<Marker, DecodedRow, Grouped> From<OwnedPreparedStatement<Marker, DecodedRow, Grouped>>
    for PreparedStatement<'_, Marker, DecodedRow, Grouped>
{
    fn from(value: OwnedPreparedStatement<Marker, DecodedRow, Grouped>) -> Self {
        let params = value.inner.params.iter().map(|param| {
            Param::new(
                param.placeholder,
                param
                    .value
                    .clone()
                    .map(|value| Cow::Owned(MySQLValue::from(value))),
            )
        });
        Self::new(CorePreparedStatement {
            text_segments: value.inner.text_segments,
            params: params.collect(),
            sql: value.inner.sql,
        })
    }
}

impl<Marker, DecodedRow, Grouped> core::fmt::Display
    for PreparedStatement<'_, Marker, DecodedRow, Grouped>
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.sql())
    }
}

impl<'q, Marker, DecodedRow, Grouped> ToSQL<'q, MySQLValue<'q>>
    for PreparedStatement<'q, Marker, DecodedRow, Grouped>
{
    fn to_sql(&self) -> drizzle_core::SQL<'q, MySQLValue<'q>> {
        self.inner.to_sql()
    }
}

impl<'q, Marker, DecodedRow, Grouped> ToSQL<'q, OwnedMySQLValue>
    for OwnedPreparedStatement<Marker, DecodedRow, Grouped>
{
    fn to_sql(&self) -> drizzle_core::SQL<'q, OwnedMySQLValue> {
        self.inner.to_sql()
    }
}

impl<'q, Marker, DecodedRow, Grouped> ToSQL<'q, MySQLValue<'q>>
    for OwnedPreparedStatement<Marker, DecodedRow, Grouped>
{
    fn to_sql(&self) -> drizzle_core::SQL<'q, MySQLValue<'q>> {
        self.inner.to_sql().map_params(MySQLValue::from)
    }
}

impl<Marker, DecodedRow, Grouped> core::fmt::Display
    for OwnedPreparedStatement<Marker, DecodedRow, Grouped>
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.sql())
    }
}
