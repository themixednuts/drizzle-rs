//! Protocol-neutral pieces shared by the blocking and Tokio MySQL adapters.

use core::marker::PhantomData;

use drizzle_core::{
    error::{DrizzleError, QueryContext, Result, ResultExt},
    row::{DecodeSelectedRef, FromDrizzleRow},
    traits::ToSQL,
};
use drizzle_mysql::{MySQLRow, values::MySQLValue};
use mysql_common::{params::Params, row::Row, value::Value};

#[derive(Debug, Clone, Copy)]
pub struct BlockingPreparedAdapter;

#[derive(Debug, Clone, Copy)]
pub struct TokioPreparedAdapter;

#[derive(Debug, Clone)]
pub struct PreparedStatement<'q, Adapter, Marker = (), DecodedRow = (), Grouped = ()> {
    pub(crate) inner: drizzle_core::prepared::PreparedStatement<'q, MySQLValue<'q>>,
    marker: core::marker::PhantomData<(Adapter, Marker, DecodedRow, Grouped)>,
}

impl<'q, Adapter, Marker, DecodedRow, Grouped>
    PreparedStatement<'q, Adapter, Marker, DecodedRow, Grouped>
{
    pub(crate) const fn new(
        inner: drizzle_core::prepared::PreparedStatement<'q, MySQLValue<'q>>,
    ) -> Self {
        Self {
            inner,
            marker: core::marker::PhantomData,
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

    #[must_use]
    pub fn into_owned(self) -> OwnedPreparedStatement<Adapter, Marker, DecodedRow, Grouped> {
        let params =
            self.inner
                .params
                .into_vec()
                .into_iter()
                .map(|param| drizzle_core::param::OwnedParam {
                    placeholder: param.placeholder,
                    value: param.value.map(|value| {
                        drizzle_mysql::values::OwnedMySQLValue::from(value.into_owned())
                    }),
                });
        OwnedPreparedStatement {
            inner: drizzle_core::prepared::OwnedPreparedStatement {
                text_segments: self.inner.text_segments,
                params: params.collect(),
                sql: self.inner.sql,
            },
            marker: core::marker::PhantomData,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OwnedPreparedStatement<Adapter, Marker = (), DecodedRow = (), Grouped = ()> {
    inner: drizzle_core::prepared::OwnedPreparedStatement<drizzle_mysql::values::OwnedMySQLValue>,
    marker: core::marker::PhantomData<(Adapter, Marker, DecodedRow, Grouped)>,
}

impl<Adapter, Marker, DecodedRow, Grouped>
    OwnedPreparedStatement<Adapter, Marker, DecodedRow, Grouped>
{
    pub(crate) fn borrowed(&self) -> PreparedStatement<'_, Adapter, Marker, DecodedRow, Grouped> {
        let params = self.inner.params.iter().map(|param| {
            drizzle_core::param::Param::new(
                param.placeholder,
                param.value.clone().map(|value| {
                    std::borrow::Cow::Owned(drizzle_mysql::values::MySQLValue::from(value))
                }),
            )
        });
        PreparedStatement::new(drizzle_core::prepared::PreparedStatement {
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
}

impl<Adapter, Marker, DecodedRow, Grouped>
    From<PreparedStatement<'_, Adapter, Marker, DecodedRow, Grouped>>
    for OwnedPreparedStatement<Adapter, Marker, DecodedRow, Grouped>
{
    fn from(value: PreparedStatement<'_, Adapter, Marker, DecodedRow, Grouped>) -> Self {
        value.into_owned()
    }
}

impl<Adapter, Marker, DecodedRow, Grouped>
    From<OwnedPreparedStatement<Adapter, Marker, DecodedRow, Grouped>>
    for PreparedStatement<'_, Adapter, Marker, DecodedRow, Grouped>
{
    fn from(value: OwnedPreparedStatement<Adapter, Marker, DecodedRow, Grouped>) -> Self {
        let params = value.inner.params.iter().map(|param| {
            drizzle_core::param::Param::new(
                param.placeholder,
                param.value.clone().map(|value| {
                    std::borrow::Cow::Owned(drizzle_mysql::values::MySQLValue::from(value))
                }),
            )
        });
        Self::new(drizzle_core::prepared::PreparedStatement {
            text_segments: value.inner.text_segments,
            params: params.collect(),
            sql: value.inner.sql,
        })
    }
}

impl<Adapter, Marker, DecodedRow, Grouped> core::fmt::Display
    for PreparedStatement<'_, Adapter, Marker, DecodedRow, Grouped>
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.sql())
    }
}

impl<'q, Adapter, Marker, DecodedRow, Grouped> drizzle_core::traits::ToSQL<'q, MySQLValue<'q>>
    for PreparedStatement<'q, Adapter, Marker, DecodedRow, Grouped>
{
    fn to_sql(&self) -> drizzle_core::SQL<'q, MySQLValue<'q>> {
        drizzle_core::traits::ToSQL::to_sql(&self.inner)
    }
}

impl<'q, Adapter, Marker, DecodedRow, Grouped>
    drizzle_core::traits::ToSQL<'q, drizzle_mysql::values::OwnedMySQLValue>
    for OwnedPreparedStatement<Adapter, Marker, DecodedRow, Grouped>
{
    fn to_sql(&self) -> drizzle_core::SQL<'q, drizzle_mysql::values::OwnedMySQLValue> {
        drizzle_core::traits::ToSQL::to_sql(&self.inner)
    }
}

impl<'q, Adapter, Marker, DecodedRow, Grouped> drizzle_core::traits::ToSQL<'q, MySQLValue<'q>>
    for OwnedPreparedStatement<Adapter, Marker, DecodedRow, Grouped>
{
    fn to_sql(&self) -> drizzle_core::SQL<'q, MySQLValue<'q>> {
        drizzle_core::traits::ToSQL::to_sql(&self.inner).map_params(MySQLValue::from)
    }
}

impl<Adapter, Marker, DecodedRow, Grouped> core::fmt::Display
    for OwnedPreparedStatement<Adapter, Marker, DecodedRow, Grouped>
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.sql())
    }
}

pub(crate) fn positional(values: impl IntoIterator<Item = Value>) -> Params {
    let values = values.into_iter().collect::<Vec<_>>();
    if values.is_empty() {
        Params::Empty
    } else {
        Params::Positional(values)
    }
}

pub(crate) fn render<'q>(query: impl ToSQL<'q, MySQLValue<'q>>) -> (String, Vec<MySQLValue<'q>>) {
    let sql = query.into_sql();
    let (text, values) = sql.build();
    (text, values.into_iter().cloned().collect())
}

pub(crate) struct QueryOutput<'q> {
    sql: String,
    values: Vec<MySQLValue<'q>>,
    rows: Vec<Row>,
}

/// Decoded MySQL rows from a fully materialized query result.
///
/// The query has finished before this iterator is created. It owns the
/// materialized driver rows and never holds a connection, transaction, or
/// native result set open while values are decoded.
pub struct Rows<R> {
    rows: std::vec::IntoIter<Row>,
    context: QueryContext,
    _marker: PhantomData<R>,
}

impl<R> Rows<R> {
    pub(crate) fn new(rows: Vec<Row>, context: QueryContext) -> Self {
        Self {
            rows: rows.into_iter(),
            context,
            _marker: PhantomData,
        }
    }
}

impl<R> Iterator for Rows<R>
where
    for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>>,
{
    type Item = Result<R>;

    fn next(&mut self) -> Option<Self::Item> {
        self.rows
            .next()
            .map(|row| R::from_row(&MySQLRow::new(&row)).with_query(|| self.context.clone()))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.rows.size_hint()
    }
}

impl<R> ExactSizeIterator for Rows<R> where for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>> {}

impl<'q> QueryOutput<'q> {
    pub(crate) fn new(sql: String, values: Vec<MySQLValue<'q>>, rows: Vec<Row>) -> Self {
        Self { sql, values, rows }
    }

    pub(crate) fn decode_all<Mk, R>(self) -> Result<Vec<R>>
    where
        for<'row> Mk: DecodeSelectedRef<&'row MySQLRow<'row, Row>, R>,
    {
        let context_values = self.values.iter().collect::<Vec<_>>();
        self.rows
            .iter()
            .map(|row| {
                let row = MySQLRow::new(row);
                <Mk as DecodeSelectedRef<&MySQLRow<'_, Row>, R>>::decode(&row)
                    .with_query(|| QueryContext::new(&self.sql, &context_values))
            })
            .collect()
    }

    pub(crate) fn decode_first<Mk, R>(self) -> Result<R>
    where
        for<'row> Mk: DecodeSelectedRef<&'row MySQLRow<'row, Row>, R>,
    {
        let row = self.rows.first().ok_or(DrizzleError::NotFound)?;
        let context_values = self.values.iter().collect::<Vec<_>>();
        let row = MySQLRow::new(row);
        <Mk as DecodeSelectedRef<&MySQLRow<'_, Row>, R>>::decode(&row)
            .with_query(|| QueryContext::new(&self.sql, &context_values))
    }

    pub(crate) fn rows<R>(self) -> Rows<R>
    where
        for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        let context_values = self.values.iter().collect::<Vec<_>>();
        let context = QueryContext::new(&self.sql, &context_values);
        Rows::new(self.rows, context)
    }

    pub(crate) fn decode_first_row<R>(self) -> Result<R>
    where
        for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        let row = self.rows.first().ok_or(DrizzleError::NotFound)?;
        let context_values = self.values.iter().collect::<Vec<_>>();
        R::from_row(&MySQLRow::new(row))
            .with_query(|| QueryContext::new(&self.sql, &context_values))
    }

    #[cfg(feature = "query")]
    pub(crate) fn decode_relational_all<Table, Relations>(
        self,
    ) -> Result<Vec<<Relations as drizzle_core::query::BuildRow<Table::Select>>::Row>>
    where
        Table: drizzle_core::query::QueryTable,
        for<'row> Table::Select: FromDrizzleRow<MySQLRow<'row, Row>>,
        Relations: drizzle_core::query::BuildRow<Table::Select>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        self.decode_relational::<Relations, Table::Select>(Table::COLUMN_NAMES.len(), |row| {
            Table::Select::from_row(row)
        })
    }

    #[cfg(feature = "query")]
    pub(crate) fn decode_relational_partial<Table, Relations>(
        self,
    ) -> Result<Vec<<Relations as drizzle_core::query::BuildRow<Table::PartialSelect>>::Row>>
    where
        Table: drizzle_core::query::QueryTable,
        Table::PartialSelect: drizzle_core::query::FromJsonObject,
        Relations: drizzle_core::query::BuildRow<Table::PartialSelect>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        use drizzle_core::query::FromJsonObject as _;

        self.decode_relational::<Relations, Table::PartialSelect>(1, |row| {
            let json = String::from_row_at(row, 0)?;
            Table::PartialSelect::from_json_str(&json, "base")
        })
    }

    #[cfg(feature = "query")]
    fn decode_relational<Relations, Base>(
        self,
        relation_offset: usize,
        mut decode_base: impl for<'row> FnMut(&MySQLRow<'row, Row>) -> Result<Base>,
    ) -> Result<Vec<<Relations as drizzle_core::query::BuildRow<Base>>::Row>>
    where
        Relations: drizzle_core::query::BuildRow<Base>,
        Relations::Store: drizzle_core::query::DeserializeStore,
    {
        use drizzle_core::query::DeserializeStore as _;

        let context_values = self.values.iter().collect::<Vec<_>>();
        self.rows
            .iter()
            .map(|raw_row| {
                let row = MySQLRow::new(raw_row);
                let base = decode_base(&row)?;
                let mut offset = relation_offset;
                let mut next_relation = || {
                    let value = Option::<String>::from_row_at(&row, offset)?;
                    offset += 1;
                    Ok(value)
                };
                let store = Relations::Store::from_json_columns(&mut next_relation)?;
                Ok(Relations::assemble(base, store))
            })
            .collect::<Result<Vec<_>>>()
            .with_query(|| QueryContext::new(&self.sql, &context_values))
    }
}
