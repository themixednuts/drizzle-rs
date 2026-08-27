//! Detached prepared statements for the blocking MySQL adapter.

use drizzle_core::{
    error::Result,
    param::ParamBind,
    row::{
        DecodeSelectedRef, MarkerAggValidFor, MarkerColumnCountValid, MarkerScopeValidFor,
        StrictDecodeMarker,
    },
};
use drizzle_mysql::{MySQLMutationResult, MySQLRow, values::MySQLValue};
use mysql::{Row, prelude::Queryable};

use crate::builder::mysql::driver_common::{
    BlockingPreparedAdapter, OwnedPreparedStatement as SharedOwnedPreparedStatement,
    PreparedStatement as SharedPreparedStatement, QueryOutput,
};

use super::{execute_request, initialize_session, query_first_request, query_request};

/// A reusable prepared MySQL statement borrowing its SQL values.
pub type PreparedStatement<'q, Marker = (), DecodedRow = (), Grouped = ()> =
    SharedPreparedStatement<'q, BlockingPreparedAdapter, Marker, DecodedRow, Grouped>;

/// Owned counterpart to [`PreparedStatement`].
pub type OwnedPreparedStatement<Marker = (), DecodedRow = (), Grouped = ()> =
    SharedOwnedPreparedStatement<BlockingPreparedAdapter, Marker, DecodedRow, Grouped>;

impl<'q, Marker, DecodedRow, Grouped> PreparedStatement<'q, Marker, DecodedRow, Grouped> {
    /// Executes the prepared statement and returns normalized mutation metadata.
    pub fn execute(
        &self,
        connection: &mut impl Queryable,
        params: impl IntoIterator<Item = ParamBind<'q, MySQLValue<'q>>>,
    ) -> Result<MySQLMutationResult> {
        initialize_session(connection)?;
        let (sql, values) = self.inner.bind(params)?;
        execute_request(connection, sql, &values.collect::<Vec<_>>())
    }

    /// Executes the prepared query and decodes every returned row.
    pub fn all<R, ScopeProof, AggProof>(
        &self,
        connection: &mut impl Queryable,
        params: impl IntoIterator<Item = ParamBind<'q, MySQLValue<'q>>>,
    ) -> Result<Vec<R>>
    where
        for<'row> Marker: DecodeSelectedRef<&'row MySQLRow<'row, Row>, R>
            + MarkerScopeValidFor<ScopeProof>
            + StrictDecodeMarker
            + MarkerColumnCountValid<MySQLRow<'row, Row>, DecodedRow, R>,
        Marker: MarkerAggValidFor<Grouped, AggProof>,
    {
        initialize_session(connection)?;
        let (sql, values) = self.inner.bind(params)?;
        let values = values.collect::<Vec<_>>();
        let rows = query_request(connection, sql, &values)?;
        QueryOutput::new(sql.to_owned(), values, rows).decode_all::<Marker, R>()
    }

    /// Executes the prepared query and decodes its first row.
    pub fn get<R, ScopeProof, AggProof>(
        &self,
        connection: &mut impl Queryable,
        params: impl IntoIterator<Item = ParamBind<'q, MySQLValue<'q>>>,
    ) -> Result<R>
    where
        for<'row> Marker: DecodeSelectedRef<&'row MySQLRow<'row, Row>, R>
            + MarkerScopeValidFor<ScopeProof>
            + StrictDecodeMarker
            + MarkerColumnCountValid<MySQLRow<'row, Row>, DecodedRow, R>,
        Marker: MarkerAggValidFor<Grouped, AggProof>,
    {
        initialize_session(connection)?;
        let (sql, values) = self.inner.bind(params)?;
        let values = values.collect::<Vec<_>>();
        let rows = query_first_request(connection, sql, &values)?
            .into_iter()
            .collect();
        QueryOutput::new(sql.to_owned(), values, rows).decode_first::<Marker, R>()
    }
}

impl<Marker, DecodedRow, Grouped> OwnedPreparedStatement<Marker, DecodedRow, Grouped> {
    /// Executes the prepared statement and returns normalized mutation metadata.
    pub fn execute<'a>(
        &'a self,
        connection: &mut impl Queryable,
        params: impl IntoIterator<Item = ParamBind<'a, MySQLValue<'a>>>,
    ) -> Result<MySQLMutationResult> {
        self.borrowed().execute(connection, params)
    }

    /// Executes the prepared query and decodes every returned row.
    pub fn all<'a, R, ScopeProof, AggProof>(
        &'a self,
        connection: &mut impl Queryable,
        params: impl IntoIterator<Item = ParamBind<'a, MySQLValue<'a>>>,
    ) -> Result<Vec<R>>
    where
        for<'row> Marker: DecodeSelectedRef<&'row MySQLRow<'row, Row>, R>
            + MarkerScopeValidFor<ScopeProof>
            + StrictDecodeMarker
            + MarkerColumnCountValid<MySQLRow<'row, Row>, DecodedRow, R>,
        Marker: MarkerAggValidFor<Grouped, AggProof>,
    {
        self.borrowed().all(connection, params)
    }

    /// Executes the prepared query and decodes its first row.
    pub fn get<'a, R, ScopeProof, AggProof>(
        &'a self,
        connection: &mut impl Queryable,
        params: impl IntoIterator<Item = ParamBind<'a, MySQLValue<'a>>>,
    ) -> Result<R>
    where
        for<'row> Marker: DecodeSelectedRef<&'row MySQLRow<'row, Row>, R>
            + MarkerScopeValidFor<ScopeProof>
            + StrictDecodeMarker
            + MarkerColumnCountValid<MySQLRow<'row, Row>, DecodedRow, R>,
        Marker: MarkerAggValidFor<Grouped, AggProof>,
    {
        self.borrowed().get(connection, params)
    }
}
