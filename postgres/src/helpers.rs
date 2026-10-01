//! SQL fragment builders used by the `PostgreSQL` query builders.
//!
//! Most items are internal. The public `*_join_using` functions render a
//! `JOIN ... USING (...)` fragment for hand-built SQL.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
use crate::traits::PostgresTable;
use crate::values::PostgresValue;
use drizzle_core::{SQL, SQLTableInfo, ToSQL, Token, helpers, traits::SQLModel};

// Re-export core helpers with PostgresValue type for convenience
pub(crate) use helpers::{
    delete, except, except_all, from, group_by_expr, having, intersect, intersect_all, limit,
    offset, order_by, select, select_distinct, set, union, union_all, update, r#where,
};

// Re-export Join from core
pub use drizzle_core::Join;

/// A table or derived table that can be joined.
#[doc(hidden)]
pub trait JoinSource<'a>: join_source_private::Sealed {
    type JoinedTable;

    fn into_join_source_sql(self) -> SQL<'a, PostgresValue<'a>>;
}

mod join_source_private {
    pub trait Sealed {}
}

impl<'a, Table> join_source_private::Sealed for Table where Table: PostgresTable<'a> {}

impl<'a, Name, Projection, Query> join_source_private::Sealed
    for drizzle_core::Derived<'a, PostgresValue<'a>, Name, Projection, Query>
where
    Name: drizzle_core::Tag,
    Projection: drizzle_core::DerivedProjection<Name>,
    Query: ToSQL<'a, PostgresValue<'a>>,
{
}

impl<'a, Table> JoinSource<'a> for Table
where
    Table: PostgresTable<'a>,
{
    type JoinedTable = Table;

    fn into_join_source_sql(self) -> SQL<'a, PostgresValue<'a>> {
        self.into_sql()
    }
}

impl<'a, Name, Projection, Query> JoinSource<'a>
    for drizzle_core::Derived<'a, PostgresValue<'a>, Name, Projection, Query>
where
    Name: drizzle_core::Tag,
    Projection: drizzle_core::DerivedProjection<Name>,
    Query: ToSQL<'a, PostgresValue<'a>>,
{
    type JoinedTable = Self;

    fn into_join_source_sql(self) -> SQL<'a, PostgresValue<'a>> {
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
    /// Sources read by the legacy `ON` predicate (see [`drizzle_core::scope`]).
    type OnSources;

    fn into_cross_join_sql(self) -> SQL<'a, PostgresValue<'a>>;
}

mod cross_join_arg_private {
    pub trait Sealed {}

    impl<'a, Source> Sealed for Source where Source: super::JoinSource<'a> {}

    impl<'a, Source, Condition> Sealed for (Source, Condition)
    where
        Source: super::JoinSource<'a>,
        Condition: drizzle_core::ToSQL<'a, crate::values::PostgresValue<'a>>,
    {
    }
}

impl<'a, Source, FromTable> CrossJoinArg<'a, FromTable> for Source
where
    Source: JoinSource<'a>,
{
    type JoinedTable = Source::JoinedTable;
    type OnSources = ();

    fn into_cross_join_sql(self) -> SQL<'a, PostgresValue<'a>> {
        Join::new()
            .cross()
            .into_sql()
            .append(self.into_join_source_sql())
    }
}

impl<'a, Source, Condition, FromTable> CrossJoinArg<'a, FromTable> for (Source, Condition)
where
    Source: JoinSource<'a>,
    Condition: ToSQL<'a, PostgresValue<'a>> + drizzle_core::expr::ExprSources,
{
    type JoinedTable = Source::JoinedTable;
    type OnSources = Condition::Sources;

    fn into_cross_join_sql(self) -> SQL<'a, PostgresValue<'a>> {
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
    table_trait: PostgresTable<'a>,
    table_info_trait: SQLTableInfo,
    condition_trait: ToSQL<'a, PostgresValue<'a>>,
    join_source_trait: JoinSource<'a>,
    value_type: PostgresValue<'a>,
);

// Generate all join helper functions using the shared macro
drizzle_core::impl_join_helpers!(
    table_trait: PostgresTable<'a>,
    condition_trait: ToSQL<'a, PostgresValue<'a>>,
    sql_type: SQL<'a, PostgresValue<'a>>,
);

/// Renders `SELECT DISTINCT ON (on) columns`.
pub(crate) fn select_distinct_on<'a, On, Columns>(
    on: On,
    columns: Columns,
) -> SQL<'a, PostgresValue<'a>>
where
    On: ToSQL<'a, PostgresValue<'a>>,
    Columns: ToSQL<'a, PostgresValue<'a>>,
{
    SQL::from_iter([Token::SELECT, Token::DISTINCT, Token::ON, Token::LPAREN])
        .append(on.into_sql())
        .push(Token::RPAREN)
        .append(columns.into_sql())
}

//------------------------------------------------------------------------------
// USING clause internal helper (PostgreSQL-specific)
//------------------------------------------------------------------------------

fn join_using_internal<'a, Table>(
    table: Table,
    join: Join,
    columns: impl ToSQL<'a, PostgresValue<'a>>,
) -> SQL<'a, PostgresValue<'a>>
where
    Table: PostgresTable<'a>,
{
    join.into_sql()
        .append(table.into_sql())
        .push(Token::USING)
        .push(Token::LPAREN)
        .append(columns.into_sql())
        .push(Token::RPAREN)
}

//------------------------------------------------------------------------------
// USING clause versions of JOIN functions (PostgreSQL-specific)
//------------------------------------------------------------------------------

/// Renders `JOIN table USING (columns)`, which joins rows whose same-named
/// columns are equal.
///
/// `columns` is rendered as given. `PostgreSQL` requires bare column names in
/// `USING`, so pass identifiers such as `SQL::ident("id")`; a table column
/// renders qualified (`"posts"."id"`), which the server rejects.
///
/// # Examples
///
/// ```rust
/// # extern crate self as drizzle;
/// # mod _drizzle {
/// #     pub mod core { pub use drizzle_core::*; }
/// #     pub mod error { pub use drizzle_core::error::*; }
/// #     pub mod types { pub use drizzle_types::*; }
/// #     pub mod migrations { pub use drizzle_migrations::*; }
/// #     pub use drizzle_types::Dialect;
/// #     pub use drizzle_types as ddl;
/// #     pub mod postgres {
/// #         pub mod values { pub use drizzle_postgres::values::*; }
/// #         pub mod traits { pub use drizzle_postgres::traits::*; }
/// #         pub mod common { pub use drizzle_postgres::common::*; }
/// #         pub mod attrs { pub use drizzle_postgres::attrs::*; }
/// #         pub mod builder { pub use drizzle_postgres::builder::*; }
/// #         pub mod helpers { pub use drizzle_postgres::helpers::*; }
/// #         pub mod expr { pub use drizzle_postgres::expr::*; }
/// #         pub mod types { pub use drizzle_postgres::types::*; }
/// #         #[cfg(feature = "aws-data-api")]
/// #         pub mod aws_data_api { pub use drizzle_postgres::aws_data_api::*; }
/// #         pub struct Row;
/// #         impl Row {
/// #             pub fn get<'a, I, T>(&'a self, _: I) -> T { unimplemented!() }
/// #             pub fn try_get<'a, I, T>(&'a self, _: I) -> Result<T, Box<dyn std::error::Error + Sync + Send>> { unimplemented!() }
/// #         }
/// #         pub mod prelude {
/// #             pub use drizzle_macros::{PostgresTable, PostgresSchema, PostgresIndex};
/// #             pub use drizzle_postgres::attrs::*;
/// #             pub use drizzle_postgres::common::PostgresSchemaType;
/// #             pub use drizzle_postgres::traits::{PostgresColumn, PostgresTable};
/// #             pub use drizzle_postgres::values::{PostgresInsertValue, PostgresUpdateValue, PostgresValue};
/// #             pub use drizzle_core::*;
/// #         }
/// #     }
/// # }
/// # pub use _drizzle::*;
/// # pub use const_format;
/// # fn main() {
/// # use drizzle::postgres::prelude::*;
/// # use drizzle::postgres::builder::QueryBuilder;
/// # #[PostgresTable(name = "users")]
/// # struct User {
/// #     #[column(serial, primary)]
/// #     id: i32,
/// #     name: String,
/// #     email: Option<String>,
/// # }
/// # #[PostgresTable(name = "posts")]
/// # struct Post {
/// #     #[column(serial, primary)]
/// #     id: i32,
/// #     #[column(references = User::id)]
/// #     author_id: i32,
/// #     title: String,
/// # }
/// # #[derive(PostgresSchema)]
/// # struct Schema {
/// #     user: User,
/// #     post: Post,
/// # }
/// # let db = QueryBuilder::new::<Schema>();
/// # let Schema { user, post } = Schema::new();
/// use drizzle::core::SQL;
/// use drizzle::postgres::values::PostgresValue;
///
/// let join = drizzle::postgres::helpers::join_using(post, SQL::<PostgresValue>::ident("id"));
/// assert_eq!(join.sql(), r#"JOIN "posts" USING ("id")"#);
/// # }
/// ```
pub fn join_using<'a, Table>(
    table: Table,
    columns: impl ToSQL<'a, PostgresValue<'a>>,
) -> SQL<'a, PostgresValue<'a>>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new(), columns)
}

/// Renders `INNER JOIN table USING (columns)`. See [`join_using`] for how to pass `columns`.
pub fn inner_join_using<'a, Table>(
    table: Table,
    columns: impl ToSQL<'a, PostgresValue<'a>>,
) -> SQL<'a, PostgresValue<'a>>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new().inner(), columns)
}

/// Renders `LEFT JOIN table USING (columns)`. See [`join_using`] for how to pass `columns`.
pub fn left_join_using<'a, Table>(
    table: Table,
    columns: impl ToSQL<'a, PostgresValue<'a>>,
) -> SQL<'a, PostgresValue<'a>>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new().left(), columns)
}

/// Renders `LEFT OUTER JOIN table USING (columns)`. See [`join_using`] for how to pass `columns`.
pub fn left_outer_join_using<'a, Table>(
    table: Table,
    columns: impl ToSQL<'a, PostgresValue<'a>>,
) -> SQL<'a, PostgresValue<'a>>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new().left().outer(), columns)
}

/// Renders `RIGHT JOIN table USING (columns)`. See [`join_using`] for how to pass `columns`.
pub fn right_join_using<'a, Table>(
    table: Table,
    columns: impl ToSQL<'a, PostgresValue<'a>>,
) -> SQL<'a, PostgresValue<'a>>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new().right(), columns)
}

/// Renders `RIGHT OUTER JOIN table USING (columns)`. See [`join_using`] for how to pass `columns`.
pub fn right_outer_join_using<'a, Table>(
    table: Table,
    columns: impl ToSQL<'a, PostgresValue<'a>>,
) -> SQL<'a, PostgresValue<'a>>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new().right().outer(), columns)
}

/// Renders `FULL JOIN table USING (columns)`. See [`join_using`] for how to pass `columns`.
pub fn full_join_using<'a, Table>(
    table: Table,
    columns: impl ToSQL<'a, PostgresValue<'a>>,
) -> SQL<'a, PostgresValue<'a>>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new().full(), columns)
}

/// Renders `FULL OUTER JOIN table USING (columns)`. See [`join_using`] for how to pass `columns`.
pub fn full_outer_join_using<'a, Table>(
    table: Table,
    columns: impl ToSQL<'a, PostgresValue<'a>>,
) -> SQL<'a, PostgresValue<'a>>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new().full().outer(), columns)
}

// Note: NATURAL JOINs don't use USING clause as they automatically match column names
// CROSS JOIN also doesn't use USING clause as it produces Cartesian product

/// Renders `INSERT INTO table`.
pub(crate) fn insert<'a, Table>(table: &Table) -> SQL<'a, PostgresValue<'a>>
where
    Table: PostgresTable<'a>,
{
    SQL::from_iter([Token::INSERT, Token::INTO]).append(table)
}

/// Creates the rows of an INSERT statement.
///
/// Rows usually set the same columns. A `None` passed to a `with_*` setter
/// leaves that column to its default without changing the row's type, so
/// rows can differ; then every row lists the union of the columns, with
/// `DEFAULT` where it sets none.
pub(crate) fn values<'a, Table, T>(
    rows: impl IntoIterator<Item = Table::Insert<T>>,
) -> SQL<'a, PostgresValue<'a>>
where
    Table: PostgresTable<'a>,
{
    let rows: Vec<_> = rows.into_iter().collect();

    if rows.is_empty() {
        return SQL::from(Token::VALUES);
    }

    let columns_info = rows[0].columns();
    let columns_slice = columns_info.as_ref();
    if rows[1..]
        .iter()
        .any(|row| row.columns().as_ref() != columns_slice)
    {
        let rows_sql = drizzle_core::helpers::insert_values_with_defaults(
            rows.iter()
                .map(|row| (row.columns(), row.values()))
                .collect(),
        );
        if let Some(rows_sql) = rows_sql {
            return rows_sql;
        }
    }

    if columns_slice.is_empty() {
        // `DEFAULT VALUES` inserts one row. A query without columns inserts
        // one all-default row per result row.
        // Raw text, not SELECT/FROM tokens: the renderer expands a bare
        // `SELECT` token followed by `FROM` into a projection.
        return if rows.len() == 1 {
            SQL::from_iter([Token::DEFAULT, Token::VALUES])
        } else {
            SQL::raw("SELECT FROM").append(SQL::func(
                "generate_series",
                SQL::number(1)
                    .push(Token::COMMA)
                    .append(SQL::number(rows.len())),
            ))
        };
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

/// Renders `RETURNING columns`, or `RETURNING *` for an empty list.
pub(crate) fn returning<'a, 'b, I>(columns: I) -> SQL<'a, PostgresValue<'a>>
where
    I: ToSQL<'a, PostgresValue<'a>>,
{
    let columns = columns.into_sql();
    let columns = if columns.chunks.is_empty() {
        SQL::from(Token::STAR)
    } else {
        columns
    };
    SQL::from(Token::RETURNING).append(columns)
}

//------------------------------------------------------------------------------
// FOR UPDATE/SHARE row locking (PostgreSQL-specific)
//------------------------------------------------------------------------------

/// Renders `FOR UPDATE`.
pub(crate) fn for_update<'a>() -> SQL<'a, PostgresValue<'a>> {
    SQL::from_iter([Token::FOR, Token::UPDATE])
}

/// Renders `FOR SHARE`.
pub(crate) fn for_share<'a>() -> SQL<'a, PostgresValue<'a>> {
    SQL::from_iter([Token::FOR, Token::SHARE])
}

/// Renders `FOR NO KEY UPDATE`.
pub(crate) fn for_no_key_update<'a>() -> SQL<'a, PostgresValue<'a>> {
    SQL::from_iter([Token::FOR, Token::NO, Token::KEY, Token::UPDATE])
}

/// Renders `FOR KEY SHARE`.
pub(crate) fn for_key_share<'a>() -> SQL<'a, PostgresValue<'a>> {
    SQL::from_iter([Token::FOR, Token::KEY, Token::SHARE])
}

/// Renders `FOR UPDATE OF table`, with the bare table name `PostgreSQL` requires.
pub(crate) fn for_update_of<'a>(table_name: &str) -> SQL<'a, PostgresValue<'a>> {
    SQL::from_iter([Token::FOR, Token::UPDATE, Token::OF])
        .append(SQL::ident(String::from(table_name)))
}

/// Renders `FOR SHARE OF table`, with the bare table name `PostgreSQL` requires.
pub(crate) fn for_share_of<'a>(table_name: &str) -> SQL<'a, PostgresValue<'a>> {
    SQL::from_iter([Token::FOR, Token::SHARE, Token::OF])
        .append(SQL::ident(String::from(table_name)))
}

/// Renders `NOWAIT`.
pub(crate) fn nowait<'a>() -> SQL<'a, PostgresValue<'a>> {
    SQL::from(Token::NOWAIT)
}

/// Renders `SKIP LOCKED`.
pub(crate) fn skip_locked<'a>() -> SQL<'a, PostgresValue<'a>> {
    SQL::from_iter([Token::SKIP, Token::LOCKED])
}
