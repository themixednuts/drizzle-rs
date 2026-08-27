//! Detached prepared statements for the Tokio MySQL adapter.

use drizzle_core::{
    error::Result,
    param::ParamBind,
    row::{
        DecodeSelectedRef, MarkerAggValidFor, MarkerColumnCountValid, MarkerScopeValidFor,
        StrictDecodeMarker,
    },
};
use drizzle_mysql::{MySQLMutationResult, MySQLRow, values::MySQLValue};
use mysql_async::{Row, prelude::ToConnection};

use crate::builder::mysql::driver_common::{
    OwnedPreparedStatement as SharedOwnedPreparedStatement,
    PreparedStatement as SharedPreparedStatement, QueryOutput, TokioPreparedAdapter,
};

use super::{execute_request, initialize_session, query_first_request, query_request};

/// A reusable prepared MySQL statement borrowing its SQL values.
pub type PreparedStatement<'q, Marker = (), DecodedRow = (), Grouped = ()> =
    SharedPreparedStatement<'q, TokioPreparedAdapter, Marker, DecodedRow, Grouped>;

/// Owned counterpart to [`PreparedStatement`].
pub type OwnedPreparedStatement<Marker = (), DecodedRow = (), Grouped = ()> =
    SharedOwnedPreparedStatement<TokioPreparedAdapter, Marker, DecodedRow, Grouped>;

impl<'q, Marker, DecodedRow, Grouped> PreparedStatement<'q, Marker, DecodedRow, Grouped> {
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
}

impl<Marker, DecodedRow, Grouped> OwnedPreparedStatement<Marker, DecodedRow, Grouped> {
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
