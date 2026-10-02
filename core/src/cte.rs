//! Driver-neutral common table expression types.

use core::{marker::PhantomData, ops::Deref};

use crate::{SQL, SQLParam, ToSQL, Token};

/// Something that can be listed in a `WITH` clause.
pub trait CTEDefinition<'a, V: SQLParam> {
    /// Renders the definition: `"name" AS (SELECT ...)`.
    fn cte_definition(&self) -> SQL<'a, V>;
}

/// A named common table expression (CTE) that can be read like a table.
///
/// Created by a SELECT builder's `.into_cte::<Tag>()`. Pass it to `.with(...)`
/// to define it, then select from it. Its typed columns are reached through
/// `table` (or directly, through `Deref`). As a source it renders as its
/// quoted name.
///
/// # Examples
///
/// ```
/// use drizzle_core::{SQL, ToSQL};
/// use drizzle_core::cte::{CTEDefinition, CTEView};
/// # use drizzle_core::{Dialect, SQLParam, SQLiteDialect};
/// # use std::borrow::Cow;
/// # #[derive(Debug, Clone, PartialEq)]
/// # struct Value(i64);
/// # impl SQLParam for Value {
/// #     const DIALECT: Dialect = Dialect::SQLite;
/// #     type DialectMarker = SQLiteDialect;
/// # }
/// # impl From<Value> for Cow<'_, Value> {
/// #     fn from(value: Value) -> Self { Cow::Owned(value) }
/// # }
///
/// let recent = CTEView::new((), "recent", SQL::<Value>::raw("SELECT 1"));
/// assert_eq!(recent.cte_definition().sql(), r#""recent" AS (SELECT 1)"#);
/// assert_eq!(recent.to_sql().sql(), r#""recent""#);
/// ```
#[derive(Clone, Debug)]
pub struct CTEView<'a, V: SQLParam, Table, Query> {
    /// The aliased table that gives typed access to the CTE's columns.
    pub table: Table,
    name: &'static str,
    query: Query,
    value: PhantomData<(&'a (), V)>,
}

impl<'a, V, Table, Query> CTEView<'a, V, Table, Query>
where
    V: SQLParam,
    Query: ToSQL<'a, V>,
{
    /// Creates a CTE named `name`, defined by `query`, with columns typed by
    /// `table`.
    pub const fn new(table: Table, name: &'static str, query: Query) -> Self {
        Self {
            table,
            name,
            query,
            value: PhantomData,
        }
    }

    /// Returns the CTE name.
    pub const fn cte_name(&self) -> &'static str {
        self.name
    }

    /// Returns the defining query.
    pub const fn query(&self) -> &Query {
        &self.query
    }
}

impl<'a, V, Table, Query> CTEDefinition<'a, V> for CTEView<'a, V, Table, Query>
where
    V: SQLParam,
    Query: ToSQL<'a, V>,
{
    fn cte_definition(&self) -> SQL<'a, V> {
        SQL::ident(self.name)
            .push(Token::AS)
            .append(self.query.to_sql().parens())
    }
}

impl<'a, V, Table, Query> CTEDefinition<'a, V> for &CTEView<'a, V, Table, Query>
where
    V: SQLParam,
    Query: ToSQL<'a, V>,
{
    fn cte_definition(&self) -> SQL<'a, V> {
        (*self).cte_definition()
    }
}

// A CTE is looked up in scope by the key of the aliased table it exposes.
impl<V: SQLParam, Table: crate::scope::ScopeEntry, Query> crate::scope::ScopeEntry
    for CTEView<'_, V, Table, Query>
{
    type Key = Table::Key;
    type Nullable = Table::Nullable;
    // The CTE query is checked where it is defined.
    type Sources = ();
}

impl<V: SQLParam, Table, Query> Deref for CTEView<'_, V, Table, Query> {
    type Target = Table;

    fn deref(&self) -> &Self::Target {
        &self.table
    }
}

impl<'a, V, Table, Query> ToSQL<'a, V> for CTEView<'a, V, Table, Query>
where
    V: SQLParam,
    Query: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::ident(self.name)
    }
}
