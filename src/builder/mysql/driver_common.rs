//! Protocol-neutral pieces shared by the blocking and Tokio MySQL adapters.

use core::marker::PhantomData;

use drizzle_core::{
    error::{DrizzleError, QueryContext, Result, ResultExt},
    row::{DecodeSelectedRef, FromDrizzleRow},
    traits::ToSQL,
};
use drizzle_mysql::{MySQLRow, values::MySQLValue};
use mysql_common::{params::Params, row::Row, value::Value};

/// The [`ErrorKind`](drizzle_core::error::ErrorKind) of a MySQL or MariaDB
/// server error, and the constraint its message names.
pub(crate) fn server_error_kind(
    code: u16,
    message: &str,
) -> (drizzle_core::error::ErrorKind, Option<&str>) {
    use drizzle_core::error::ErrorKind;

    let between = |start: &str, end: char| {
        message
            .split_once(start)
            .and_then(|(_, rest)| rest.split_once(end))
            .map(|(name, _)| name)
    };
    match code {
        // ER_DUP_ENTRY, ER_DUP_ENTRY_WITH_KEY_NAME: `... for key 'users.users_email_unique'`
        // (MySQL 8 writes the table before the key; MariaDB does not). The
        // duplicate value comes first, unescaped, so the key is the last match.
        1062 | 1586 => (
            ErrorKind::UniqueViolation,
            message
                .rsplit_once("for key '")
                .and_then(|(_, rest)| rest.split_once('\''))
                .map(|(key, _)| key.rsplit('.').next().unwrap_or(key)),
        ),
        // ER_ROW_IS_REFERENCED(_2), ER_NO_REFERENCED_ROW(_2): `... CONSTRAINT `name` FOREIGN KEY ...`
        1216 | 1217 | 1451 | 1452 => (ErrorKind::ForeignKeyViolation, between("CONSTRAINT `", '`')),
        // ER_BAD_NULL_ERROR, ER_NO_DEFAULT_FOR_FIELD
        1048 | 1364 => (ErrorKind::NotNullViolation, None),
        // ER_CHECK_CONSTRAINT_VIOLATED: `Check constraint 'name' is violated.`
        3819 => (
            ErrorKind::CheckViolation,
            between("Check constraint '", '\''),
        ),
        // MariaDB's ER_CONSTRAINT_FAILED: `CONSTRAINT `name` failed for ...`
        4025 => (ErrorKind::CheckViolation, between("CONSTRAINT `", '`')),
        // ER_LOCK_DEADLOCK
        1213 => (ErrorKind::Deadlock, None),
        // ER_CHECKREAD: MariaDB's snapshot isolation saw a row change after
        // the transaction's snapshot.
        1020 => (ErrorKind::SerializationFailure, None),
        // ER_LOCK_WAIT_TIMEOUT, ER_LOCK_NOWAIT (`NOWAIT` found the row locked)
        1205 | 3572 => (ErrorKind::Busy, None),
        _ => (ErrorKind::Other, None),
    }
}

/// Marks a prepared statement for the blocking `mysql` adapter.
#[derive(Debug, Clone, Copy)]
pub struct BlockingPreparedAdapter;

/// Marks a prepared statement for the Tokio `mysql_async` adapter.
#[derive(Debug, Clone, Copy)]
pub struct TokioPreparedAdapter;

/// A query rendered once, ready to run many times with new placeholder values.
///
/// Made by `.prepare()` on a query builder; each adapter's `prepared` module
/// names it for that adapter and adds `execute`, `all`, and `get`. It borrows
/// the values embedded in the query; call [`into_owned`](Self::into_owned) to
/// store it.
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

    /// Returns the rendered SQL, with `?` placeholders.
    #[must_use]
    pub fn sql(&self) -> &str {
        self.inner.sql()
    }

    /// Returns how many placeholder bindings each run expects.
    #[must_use]
    pub fn param_count(&self) -> usize {
        self.inner.external_param_count()
    }

    /// Copies the embedded values so the statement no longer borrows them.
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

/// A [`PreparedStatement`] that owns its embedded values, so it can be stored
/// or moved freely.
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

    /// Returns the rendered SQL, with `?` placeholders.
    #[must_use]
    pub fn sql(&self) -> &str {
        self.inner.sql()
    }

    /// Returns how many placeholder bindings each run expects.
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

    pub(crate) fn decode_first<Mk, R>(self) -> Result<Option<R>>
    where
        for<'row> Mk: DecodeSelectedRef<&'row MySQLRow<'row, Row>, R>,
    {
        let Some(row) = self.rows.first() else {
            return Ok(None);
        };
        let context_values = self.values.iter().collect::<Vec<_>>();
        let row = MySQLRow::new(row);
        <Mk as DecodeSelectedRef<&MySQLRow<'_, Row>, R>>::decode(&row)
            .map(Some)
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

    pub(crate) fn decode_first_row<R>(self) -> Result<Option<R>>
    where
        for<'row> R: FromDrizzleRow<MySQLRow<'row, Row>>,
    {
        let Some(row) = self.rows.first() else {
            return Ok(None);
        };
        let context_values = self.values.iter().collect::<Vec<_>>();
        R::from_row(&MySQLRow::new(row))
            .map(Some)
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

#[cfg(test)]
mod error_kind_tests {
    use super::server_error_kind;
    use drizzle_core::error::ErrorKind;

    #[test]
    fn mysql_and_mariadb_errors_name_their_kind_and_constraint() {
        let cases = [
            // MySQL 8 writes the table before the key; MariaDB does not.
            (
                1062,
                "Duplicate entry 'a@b.c' for key 'users.users_email_unique'",
                ErrorKind::UniqueViolation,
                Some("users_email_unique"),
            ),
            (
                1062,
                "Duplicate entry 'a@b.c' for key 'users_email_unique'",
                ErrorKind::UniqueViolation,
                Some("users_email_unique"),
            ),
            // The duplicate value is not escaped, so it can hold `for key '`.
            (
                1062,
                "Duplicate entry 'x' for key 'admins_pk' for key 'users.users_email_unique'",
                ErrorKind::UniqueViolation,
                Some("users_email_unique"),
            ),
            (
                1452,
                "Cannot add or update a child row: a foreign key constraint fails \
                 (`app`.`posts`, CONSTRAINT `posts_author_fk` FOREIGN KEY (`author_id`) \
                 REFERENCES `users` (`id`))",
                ErrorKind::ForeignKeyViolation,
                Some("posts_author_fk"),
            ),
            (
                1451,
                "Cannot delete or update a parent row: a foreign key constraint fails \
                 (`app`.`posts`, CONSTRAINT `posts_author_fk` FOREIGN KEY (`author_id`) \
                 REFERENCES `users` (`id`))",
                ErrorKind::ForeignKeyViolation,
                Some("posts_author_fk"),
            ),
            (
                1048,
                "Column 'name' cannot be null",
                ErrorKind::NotNullViolation,
                None,
            ),
            (
                3819,
                "Check constraint 'users_age_check' is violated.",
                ErrorKind::CheckViolation,
                Some("users_age_check"),
            ),
            (
                4025,
                "CONSTRAINT `users_age_check` failed for `app`.`users`",
                ErrorKind::CheckViolation,
                Some("users_age_check"),
            ),
            (
                1213,
                "Deadlock found when trying to get lock; try restarting transaction",
                ErrorKind::Deadlock,
                None,
            ),
            (
                1205,
                "Lock wait timeout exceeded; try restarting transaction",
                ErrorKind::Busy,
                None,
            ),
            (
                3572,
                "Statement aborted because lock(s) could not be acquired immediately \
                 and NOWAIT is set.",
                ErrorKind::Busy,
                None,
            ),
            (
                1020,
                "Record has changed since last read in table 'users'",
                ErrorKind::SerializationFailure,
                None,
            ),
            (
                1146,
                "Table 'app.missing' doesn't exist",
                ErrorKind::Other,
                None,
            ),
        ];
        for (code, message, kind, constraint) in cases {
            assert_eq!(
                server_error_kind(code, message),
                (kind, constraint),
                "{code}: {message}"
            );
        }
    }
}
