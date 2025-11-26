use crate::{SQL, SQLParam, ToSQL, Token};

/// A Common Table Expression (CTE) builder
#[derive(Clone, Debug)]
pub struct CTE {
    pub(crate) name: &'static str,
}

/// A defined CTE with its query
#[derive(Clone, Debug)]
pub struct DefinedCTE<'a, V: SQLParam, Q: ToSQL<'a, V>> {
    name: &'a str,
    query: Q,
    _phantom: core::marker::PhantomData<V>,
}

impl CTE {
    /// Define the CTE with a query
    pub fn r#as<'a, V: SQLParam, Q: ToSQL<'a, V>>(self, query: Q) -> DefinedCTE<'a, V, Q> {
        DefinedCTE {
            name: self.name,
            query,
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<'a, V: SQLParam, Q: ToSQL<'a, V>> DefinedCTE<'a, V, Q> {
    /// Get the CTE name for referencing in queries
    pub fn name(&self) -> &'a str {
        self.name
    }
}

impl<'a, V: SQLParam + 'a, Q: ToSQL<'a, V>> ToSQL<'a, V> for DefinedCTE<'a, V, Q> {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::raw(self.name)
    }
}

impl<'a, V: SQLParam, Q: ToSQL<'a, V>> DefinedCTE<'a, V, Q> {
    /// Get the full CTE definition for WITH clauses
    pub fn definition(&self) -> SQL<'a, V> {
        SQL::raw(self.name)
            .push(Token::AS)
            .append(self.query.to_sql().parens())
    }
}

impl<'a, V: SQLParam, Q: ToSQL<'a, V>> AsRef<DefinedCTE<'a, V, Q>> for DefinedCTE<'a, V, Q> {
    fn as_ref(&self) -> &DefinedCTE<'a, V, Q> {
        self
    }
}
