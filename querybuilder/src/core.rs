use std::{borrow::Cow, fmt::Display};

pub mod expressions;
pub mod traits;

pub trait SQLParam: Display + Clone {}

// Core SQL struct that holds a SQL query and its parameters
// Cow<'a, str> allows for both static or dynamically generated SQL strings
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct SQL<'a, V: SQLParam>(pub(crate) Cow<'a, str>, pub(crate) Vec<V>);

impl<'a, V: SQLParam> SQL<'a, V> {
    // Create a new SQL query with parameters from a string and parameters
    pub fn new<S: Into<Cow<'a, str>>>(sql: S, params: Vec<V>) -> Self {
        Self(sql.into(), params)
    }

    // Get the SQL query string
    pub fn sql(&self) -> &str {
        self.0.as_ref()
    }

    // Get the parameters
    pub fn params(&self) -> &[V] {
        &self.1
    }

    // Consume the SQL and return the query string and parameters
    pub fn into_parts(self) -> (Cow<'a, str>, Vec<V>) {
        (self.0, self.1)
    }
}

// Implementation of ToSQL for SQL itself (identity function)
impl<'a, V: SQLParam> ToSQL<'a, V> for SQL<'a, V> {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL(self.0.clone(), self.1.clone())
    }
}

// ToSQL trait defines how to convert a value into an SQL expression
pub trait ToSQL<'a, V: SQLParam> {
    fn to_sql(&self) -> SQL<'a, V>;
}

// Implementation of ToSQL for string literals
impl<'a, V: SQLParam> ToSQL<'a, V> for &'a str {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL(Cow::Borrowed(*self), Vec::new())
    }
}

impl<'a, V: SQLParam> Display for SQL<'a, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
// Trait for converting a value into a database parameter value
pub trait IntoValue<V> {
    fn into_value(self) -> V;
}

// Helper function to handle parameters in SQL queries
pub fn to_value<V, T: IntoValue<V>>(value: T) -> V {
    value.into_value()
}

// SQL template for creating parameterized SQL queries
pub fn sql_template<'a, V: SQLParam>(template: &'a str, params: Vec<V>) -> SQL<'a, V> {
    SQL(Cow::Borrowed(template), params)
}

// Convenience macro for SQL expressions
#[macro_export]
macro_rules! sql {
    ($sql:expr) => {
        $crate::core::SQL(std::borrow::Cow::Borrowed($sql), Vec::new())
    };
    ($sql:expr, $($param:expr),*) => {
        $crate::core::SQL(std::borrow::Cow::Borrowed($sql), vec![$($param),*])
    };
}

// Raw SQL expression (no escaping)
#[macro_export]
macro_rules! raw {
    ($value:expr) => {
        $crate::core::SQL(std::borrow::Cow::Owned($value.to_string()), Vec::new())
    };
}

// Join SQL fragments with a separator
#[macro_export]
macro_rules! join {
    ($sep:expr, $($sql:expr),*) => {{
        let fragments = vec![$($sql),*];
        let joined = fragments.iter()
            .map(|f| f.sql())
            .collect::<Vec<_>>()
            .join($sep);
        let mut params = Vec::new();
        for f in fragments {
            params.extend(f.params().to_owned());
        }
        $crate::core::SQL(std::borrow::Cow::Owned(joined), params)
    }};
}

// Placeholder for named parameters
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct Placeholder<'a>(pub &'a str);

impl<'a> Placeholder<'a> {
    pub const fn new(name: &'a str) -> Self {
        Self(name)
    }
}

impl<'a> Display for Placeholder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ":{}", self.0)
    }
}

impl<'a, V: SQLParam> ToSQL<'a, V> for Placeholder<'a> {
    fn to_sql(&self) -> SQL<'a, V> {
        let sql = format!(":{}", self.0);
        SQL(sql.into(), vec![])
    }
}

#[macro_export]
macro_rules! placeholder {
    ($name:expr) => {
        $crate::core::Placeholder($name)
    };
}

// SQL utilities
pub mod sql_utils {
    use super::*;

    // Creates an identifier SQL
    pub fn ident<'a, V: SQLParam>(name: &'a str) -> SQL<'a, V> {
        SQL(Cow::Borrowed(name), Vec::new())
    }

    // Creates a parameter with placeholder
    pub fn param<'a, V: SQLParam>(value: V) -> SQL<'a, V> {
        SQL(Cow::Borrowed("?"), vec![value])
    }
}
