//! Detached prepared statements for the blocking MySQL adapter.

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
use mysql::{Row, prelude::Queryable};

use super::{QueryOutput, execute_request, initialize_session, query_first_request, query_request};

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

    /// Rendered SQL containing positional `?` placeholders.
    #[must_use]
    pub fn sql(&self) -> &str {
        self.inner.sql()
    }

    /// Number of external bindings required by this statement.
    #[must_use]
    pub fn param_count(&self) -> usize {
        self.inner.external_param_count()
    }

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

    /// Converts this statement to an owned reusable value.
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

    /// Rendered SQL containing positional `?` placeholders.
    #[must_use]
    pub fn sql(&self) -> &str {
        self.inner.sql()
    }

    /// Number of external bindings required by this statement.
    #[must_use]
    pub fn param_count(&self) -> usize {
        self.inner.external_param_count()
    }

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

impl<'q, Marker, DecodedRow, Grouped> drizzle_core::traits::ToSQL<'q, MySQLValue<'q>>
    for PreparedStatement<'q, Marker, DecodedRow, Grouped>
{
    fn to_sql(&self) -> drizzle_core::SQL<'q, MySQLValue<'q>> {
        self.inner.to_sql()
    }
}

impl<'q, Marker, DecodedRow, Grouped> drizzle_core::traits::ToSQL<'q, OwnedMySQLValue>
    for OwnedPreparedStatement<Marker, DecodedRow, Grouped>
{
    fn to_sql(&self) -> drizzle_core::SQL<'q, OwnedMySQLValue> {
        self.inner.to_sql()
    }
}

impl<'q, Marker, DecodedRow, Grouped> drizzle_core::traits::ToSQL<'q, MySQLValue<'q>>
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
