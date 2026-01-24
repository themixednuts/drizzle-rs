use crate::values::PostgresValue;
use drizzle_core::{SQL, ToSQL, Token};
use std::marker::PhantomData;
use std::ops::Deref;

/// Trait for types that can provide a CTE definition for WITH clauses.
pub trait CTEDefinition<'a> {
    /// Returns the SQL for the CTE definition (e.g., "cte_name AS (SELECT ...)")
    fn cte_definition(&self) -> SQL<'a, PostgresValue<'a>>;
}

/// A CTE (Common Table Expression) view that wraps an aliased table with its defining query.
///
/// This struct enables type-safe CTE usage by combining:
/// - An aliased table instance for typed field access
/// - The CTE definition (query) for the WITH clause
///
/// # Example
///
/// ```rust,ignore
/// let active_users = builder
///     .select((user.id, user.name))
///     .from(user)
///     .as_cte("active_users");
///
/// // active_users.name works via Deref to the aliased table
/// builder
///     .with(&active_users)
///     .select(active_users.name)
///     .from(&active_users)
/// ```
#[derive(Clone, Debug)]
pub struct CTEView<'a, Table, Query> {
    /// The aliased table for typed field access
    pub table: Table,
    /// The CTE name
    name: &'static str,
    /// The defining query
    query: Query,
    /// Lifetime marker
    _phantom: PhantomData<PostgresValue<'a>>,
}

impl<'a, Table, Query> CTEView<'a, Table, Query>
where
    Query: ToSQL<'a, PostgresValue<'a>>,
{
    /// Creates a new CTEView with the given aliased table, name, and query.
    pub fn new(table: Table, name: &'static str, query: Query) -> Self {
        Self {
            table,
            name,
            query,
            _phantom: PhantomData,
        }
    }

    /// Returns the CTE name.
    pub fn cte_name(&self) -> &'static str {
        self.name
    }

    /// Returns a reference to the underlying query.
    pub fn query(&self) -> &Query {
        &self.query
    }
}

/// CTEDefinition implementation for CTEView.
impl<'a, Table, Query> CTEDefinition<'a> for CTEView<'a, Table, Query>
where
    Query: ToSQL<'a, PostgresValue<'a>>,
{
    fn cte_definition(&self) -> SQL<'a, PostgresValue<'a>> {
        SQL::raw(self.name)
            .push(Token::AS)
            .append(self.query.to_sql().parens())
    }
}

/// CTEDefinition for references.
impl<'a, Table, Query> CTEDefinition<'a> for &CTEView<'a, Table, Query>
where
    Query: ToSQL<'a, PostgresValue<'a>>,
{
    fn cte_definition(&self) -> SQL<'a, PostgresValue<'a>> {
        SQL::raw(self.name)
            .push(Token::AS)
            .append(self.query.to_sql().parens())
    }
}

/// Deref to the aliased table for field access.
impl<'a, Table, Query> Deref for CTEView<'a, Table, Query> {
    type Target = Table;

    fn deref(&self) -> &Self::Target {
        &self.table
    }
}

/// ToSQL implementation renders just the CTE name for use in FROM clauses.
/// Unlike aliased tables (which render as "original" AS "alias"), CTEs
/// should just render as their name since they're already defined in the WITH clause.
impl<'a, Table, Query> ToSQL<'a, PostgresValue<'a>> for CTEView<'a, Table, Query>
where
    Query: ToSQL<'a, PostgresValue<'a>>,
{
    fn to_sql(&self) -> SQL<'a, PostgresValue<'a>> {
        // Just output the CTE name - it's already defined in the WITH clause
        SQL::ident(self.name)
    }
}
