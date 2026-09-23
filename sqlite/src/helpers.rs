#[cfg(not(feature = "std"))]
use crate::prelude::*;
use crate::traits::SQLiteTable;
use crate::values::SQLiteValue;
use drizzle_core::{
    SQL, SQLChunk, Token, helpers as core_helpers,
    traits::{SQLModel, ToSQL},
};

// Re-export core helpers with SQLiteValue type for convenience
pub(crate) use core_helpers::{
    delete, except, from, group_by_expr, having, insert, intersect, limit, offset, order_by,
    select, select_distinct, set, union, union_all, update, r#where,
};

// Re-export Join from core
pub use drizzle_core::Join;

/// A table-like source accepted by an explicit JOIN tuple.
#[doc(hidden)]
pub trait JoinSource<'a>: join_source_private::Sealed {
    type JoinedTable;

    fn into_join_source_sql(self) -> SQL<'a, SQLiteValue<'a>>;
}

mod join_source_private {
    pub trait Sealed {}
}

impl<'a, Table> join_source_private::Sealed for Table where Table: SQLiteTable<'a> {}

impl<'a, Name, Projection, Query> join_source_private::Sealed
    for drizzle_core::Derived<'a, SQLiteValue<'a>, Name, Projection, Query>
where
    Name: drizzle_core::Tag,
    Projection: drizzle_core::DerivedProjection<Name>,
    Query: ToSQL<'a, SQLiteValue<'a>>,
{
}

impl<'a, Table> JoinSource<'a> for Table
where
    Table: SQLiteTable<'a>,
{
    type JoinedTable = Table;

    fn into_join_source_sql(self) -> SQL<'a, SQLiteValue<'a>> {
        self.into_sql()
    }
}

impl<'a, Name, Projection, Query> JoinSource<'a>
    for drizzle_core::Derived<'a, SQLiteValue<'a>, Name, Projection, Query>
where
    Name: drizzle_core::Tag,
    Projection: drizzle_core::DerivedProjection<Name>,
    Query: ToSQL<'a, SQLiteValue<'a>>,
{
    type JoinedTable = Self;

    fn into_join_source_sql(self) -> SQL<'a, SQLiteValue<'a>> {
        self.into_sql()
    }
}

/// A source or legacy tuple accepted by [`crate::builder::SelectBuilder::cross_join`].
///
/// A bare source renders `CROSS JOIN`. The legacy `(source, predicate)`
/// form renders the equivalent portable `INNER JOIN ... ON ...`, because
/// PostgreSQL does not allow an `ON` clause after `CROSS JOIN`.
#[doc(hidden)]
pub trait CrossJoinArg<'a, FromTable>: cross_join_arg_private::Sealed {
    type JoinedTable;

    fn into_cross_join_sql(self) -> SQL<'a, SQLiteValue<'a>>;
}

mod cross_join_arg_private {
    pub trait Sealed {}

    impl<'a, Source> Sealed for Source where Source: super::JoinSource<'a> {}

    impl<'a, Source, Condition> Sealed for (Source, Condition)
    where
        Source: super::JoinSource<'a>,
        Condition: drizzle_core::ToSQL<'a, crate::values::SQLiteValue<'a>>,
    {
    }
}

impl<'a, Source, FromTable> CrossJoinArg<'a, FromTable> for Source
where
    Source: JoinSource<'a>,
{
    type JoinedTable = Source::JoinedTable;

    fn into_cross_join_sql(self) -> SQL<'a, SQLiteValue<'a>> {
        Join::new()
            .cross()
            .into_sql()
            .append(self.into_join_source_sql())
    }
}

impl<'a, Source, Condition, FromTable> CrossJoinArg<'a, FromTable> for (Source, Condition)
where
    Source: JoinSource<'a>,
    Condition: ToSQL<'a, SQLiteValue<'a>>,
{
    type JoinedTable = Source::JoinedTable;

    fn into_cross_join_sql(self) -> SQL<'a, SQLiteValue<'a>> {
        let (source, condition) = self;
        Join::new()
            .inner()
            .into_sql()
            .append(source.into_join_source_sql())
            .push(Token::ON)
            .append(condition.into_sql())
    }
}

drizzle_core::impl_join_arg_trait!(
    table_trait: SQLiteTable<'a>,
    table_info_trait: drizzle_core::SQLTableInfo,
    condition_trait: ToSQL<'a, SQLiteValue<'a>>,
    join_source_trait: JoinSource<'a>,
    value_type: SQLiteValue<'a>,
);

// Generate all join helper functions using the shared macro
drizzle_core::impl_join_helpers!(
    table_trait: SQLiteTable<'a>,
    condition_trait: ToSQL<'a, SQLiteValue<'a>>,
    sql_type: SQL<'a, SQLiteValue<'a>>,
);

/// Creates a VALUES clause for INSERT statements.
/// All rows must declare the same set of columns.
pub(crate) fn values<'a, Table, T>(
    rows: impl IntoIterator<Item = Table::Insert<T>>,
) -> SQL<'a, SQLiteValue<'a>>
where
    Table: SQLiteTable<'a> + Default,
{
    let rows: Vec<Table::Insert<T>> = rows.into_iter().collect();

    if rows.is_empty() {
        return SQL::from(Token::VALUES);
    }

    // Since all rows have the same PATTERN, they all have the same columns
    // Get column info from the first row (all rows will have the same columns)
    let columns_info = rows[0].columns();
    let columns_slice = columns_info.as_ref();

    // Every column takes its default. `DEFAULT VALUES` inserts one row, and
    // SQLite has no `DEFAULT` keyword inside VALUES, so several such rows
    // insert NULL into `rowid`, which assigns the next rowid and leaves every
    // declared column to its default. (A WITHOUT ROWID table rejects this
    // with "no column named rowid" instead of inserting a single row.)
    if columns_slice.is_empty() {
        if rows.len() == 1 {
            return SQL::from_iter([Token::DEFAULT, Token::VALUES]);
        }
        let mut values_sql = SQL::with_capacity_chunks(rows.len().saturating_mul(4));
        for index in 0..rows.len() {
            if index > 0 {
                values_sql.push_mut(Token::COMMA);
            }
            values_sql.append_mut(SQL::from(Token::NULL).parens());
        }
        return SQL::raw("rowid")
            .parens()
            .push(Token::VALUES)
            .append(values_sql);
    }

    let columns_sql = SQL::columns(columns_slice);
    let mut values_sql = SQL::with_capacity_chunks(rows.len().saturating_mul(4));
    for (idx, row) in rows.iter().enumerate() {
        if idx > 0 {
            values_sql.push_mut(Token::COMMA);
        }
        values_sql.push_mut(Token::LPAREN);
        values_sql.append_mut(row.values());
        values_sql.push_mut(Token::RPAREN);
    }

    columns_sql.parens().push(Token::VALUES).append(values_sql)
}

/// An `OFFSET` for a query without a `LIMIT`.
///
/// `SQLite` only accepts `OFFSET` as part of a `LIMIT` clause; a negative
/// limit means "no limit".
#[track_caller]
pub(crate) fn standalone_offset<'a, P>(offset: P) -> SQL<'a, SQLiteValue<'a>>
where
    P: drizzle_core::PaginationArg<'a, SQLiteValue<'a>>,
{
    SQL::from(Token::LIMIT)
        .append(SQL::raw("-1"))
        .append(core_helpers::offset(offset))
}

/// Ends an `INSERT ... SELECT` so an upsert clause can follow it.
///
/// When the final `SELECT` ends in its `FROM` clause, SQLite parses the `ON`
/// of `ON CONFLICT` as a join constraint and rejects the statement. A
/// trailing `WHERE true` closes the `SELECT`, as SQLite's documentation
/// recommends. Inserts from VALUES, and `SELECT`s that already end in a
/// `WHERE`, `GROUP BY`, `HAVING`, `WINDOW`, `ORDER BY` or `LIMIT`, are
/// returned unchanged.
pub(crate) fn before_upsert<'a>(sql: SQL<'a, SQLiteValue<'a>>) -> SQL<'a, SQLiteValue<'a>> {
    let mut depth = 0usize;
    let mut ends_in_from = false;
    for chunk in &sql.chunks {
        match chunk {
            SQLChunk::Token(Token::LPAREN) => depth += 1,
            SQLChunk::Token(Token::RPAREN) => depth = depth.saturating_sub(1),
            SQLChunk::Token(Token::SELECT) if depth == 0 => ends_in_from = false,
            SQLChunk::Token(Token::FROM) if depth == 0 => ends_in_from = true,
            SQLChunk::Token(
                Token::WHERE
                | Token::GROUP
                | Token::HAVING
                | Token::WINDOW
                | Token::ORDER
                | Token::LIMIT,
            ) if depth == 0 => ends_in_from = false,
            _ => {}
        }
    }
    if ends_in_from {
        sql.push(Token::WHERE).append(SQL::raw("true"))
    } else {
        sql
    }
}

/// Helper function to create a RETURNING clause - `SQLite` specific
pub(crate) fn returning<'a, 'b, I>(columns: I) -> SQL<'a, SQLiteValue<'a>>
where
    I: ToSQL<'a, SQLiteValue<'a>>,
{
    let columns = columns.into_sql();
    let columns = if columns.chunks.is_empty() {
        SQL::from(Token::STAR)
    } else {
        columns
    };
    SQL::from(Token::RETURNING).append(columns)
}
