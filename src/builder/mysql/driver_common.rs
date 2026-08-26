//! Protocol-neutral pieces shared by the blocking and Tokio MySQL adapters.

use drizzle_core::{
    error::{DrizzleError, QueryContext, Result, ResultExt},
    row::{DecodeSelectedRef, FromDrizzleRow},
    traits::ToSQL,
};
use drizzle_mysql::{MySQLRow, values::MySQLValue};
use mysql_common::{params::Params, row::Row, value::Value};

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

    pub(crate) fn decode_all_rows<R>(self) -> Result<Vec<R>>
    where
        for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        let context_values = self.values.iter().collect::<Vec<_>>();
        self.rows
            .iter()
            .map(|row| {
                R::from_row(&MySQLRow::new(row))
                    .with_query(|| QueryContext::new(&self.sql, &context_values))
            })
            .collect()
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
}
