//! Driver-neutral common table expression types.

use core::{marker::PhantomData, ops::Deref};

use crate::{SQL, SQLParam, ToSQL, Token};

/// A value that can provide a CTE definition for a `WITH` clause.
pub trait CTEDefinition<'a, V: SQLParam> {
    /// Returns SQL such as `cte_name AS (SELECT ...)`.
    fn cte_definition(&self) -> SQL<'a, V>;
}

/// A CTE view with typed table projection.
#[derive(Clone, Debug)]
pub struct CTEView<'a, V: SQLParam, Table, Query> {
    /// The aliased table used for typed field access.
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
    /// Creates a CTE view.
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
