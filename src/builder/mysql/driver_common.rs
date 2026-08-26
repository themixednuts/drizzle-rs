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
