pub mod error;
pub mod expressions;
pub mod traits;

use traits::SQLParam;
// Re-export key traits from traits module
pub use traits::{IsInSchema, SQLColumn, SQLSchema, SQLTable};
use uuid::Uuid;

// TODO: Figure out the best way to incorp dialect in our types?
pub mod dialect {
    /// Represents a SQL dialect
    pub enum Dialect {
        SQLite,
        PostgreSQL,
        MySQL,
    }
}

/// The type of SQLite database object
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub enum SQLSchemaType {
    /// A regular table
    Table,
    /// A view
    View,
    /// An index
    Index,
    /// A trigger
    Trigger,
}
use std::{borrow::Cow, fmt, fmt::Display};

// Re-export common macros
#[macro_export]
macro_rules! and {
    ($expr:expr) => {
        $expr
    };

    ($($expr:expr),+ $(,)?) => {
        {
            let exprs = vec![$($expr),+];
            $crate::expressions::conditions::and(exprs.into_iter().map(Some).collect())
        }
    };
}

#[macro_export]
macro_rules! or {
    ($expr:expr) => {
        $expr
    };

    ($($expr:expr),+ $(,)?) => {
        {
            let exprs = vec![$($expr),+];
            $crate::expressions::conditions::or(exprs)
        }
    };
}

/// Join types for SQL joins
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Join {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for Join {
    fn to_sql(&self) -> SQL<'a, V> {
        let sql_str = match self {
            Join::Inner => "INNER JOIN",
            Join::Left => "LEFT JOIN",
            Join::Right => "RIGHT JOIN",
            Join::Full => "FULL JOIN", // Consider dialect differences later
            Join::Cross => "CROSS JOIN",
        };
        SQL::raw(sql_str)
    }
}

/// Sort direction for ORDER BY clauses
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for SortDirection {
    fn to_sql(&self) -> SQL<'a, V> {
        let sql_str = match self {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        };
        SQL::raw(sql_str)
    }
}

/// Various styles of SQL parameter placeholders.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceholderStyle {
    /// Colon style placeholders (:param)
    Colon,
    /// At-sign style placeholders (@param)
    AtSign,
    /// Dollar style placeholders ($param)
    Dollar,
    #[default]
    Positional,
}

/// A SQL parameter placeholder.
#[derive(Default, Debug, Clone)]
pub struct Placeholder {
    /// The name of the parameter.
    pub name: Option<&'static str>,
    /// The style of the placeholder.
    pub style: PlaceholderStyle,
}

impl Placeholder {
    /// Creates a new placeholder with the given name and style.
    pub fn with_style(name: &'static str, style: PlaceholderStyle) -> Self {
        Placeholder {
            name: Some(name),
            style,
        }
    }

    /// Creates a new colon-style placeholder.
    pub fn colon(name: &'static str) -> Self {
        Self::with_style(name, PlaceholderStyle::Colon)
    }

    /// Creates a new at-sign-style placeholder.
    pub fn at(name: &'static str) -> Self {
        Self::with_style(name, PlaceholderStyle::AtSign)
    }

    /// Creates a new dollar-style placeholder.
    pub fn dollar(name: &'static str) -> Self {
        Self::with_style(name, PlaceholderStyle::Dollar)
    }

    pub fn positional() -> Self {
        Self::default()
    }
}

impl fmt::Display for Placeholder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.style {
            PlaceholderStyle::Colon => write!(f, ":{}", self.name.unwrap_or_default()),
            PlaceholderStyle::AtSign => write!(f, "@{}", self.name.unwrap_or_default()),
            PlaceholderStyle::Dollar => write!(f, "${}", self.name.unwrap_or_default()),
            PlaceholderStyle::Positional => write!(f, "?"),
        }
    }
}

/// A SQL chunk represents a part of an SQL statement.
#[derive(Debug, Clone)]
pub enum SQLChunk<'a, V: SQLParam + 'a> {
    Text(Cow<'a, str>),
    Param {
        value: Cow<'a, V>,
        placeholder: Cow<'a, Placeholder>,
    },
    SQL(SQL<'a, V>),
}

/// A SQL statement or fragment with parameters.
///
/// This type is used to build SQL statements with proper parameter handling.
/// It keeps track of both the SQL text and the parameters to be bound.
#[derive(Debug, Clone, Default)]
pub struct SQL<'a, V: SQLParam + 'a> {
    /// The chunks that make up this SQL statement or fragment.
    pub chunks: Vec<SQLChunk<'a, V>>,
}

impl<'a, V: SQLParam + 'a> SQL<'a, V> {
    /// Creates a new SQL fragment from a raw string.
    ///
    /// The string is treated as literal SQL text, not a parameter.
    pub fn raw<T: Into<Cow<'a, str>>>(sql: T) -> Self {
        Self {
            chunks: vec![SQLChunk::Text(sql.into())],
        }
    }

    /// Creates a new SQL fragment representing a parameter.
    ///
    /// A default positional placeholder ('?') is used, and the provided value
    /// is stored for later binding.
    pub fn parameter(param: impl Into<Cow<'a, V>>) -> Self {
        Self {
            chunks: vec![SQLChunk::Param {
                value: param.into(),
                placeholder: Cow::Owned(Placeholder::positional()),
            }],
        }
    }

    /// Appends a raw string to this SQL fragment.
    ///
    /// The string is treated as literal SQL text, not a parameter.
    pub fn append_raw(mut self, sql: impl Into<Cow<'a, str>>) -> Self {
        self.chunks.push(SQLChunk::Text(sql.into()));
        self
    }

    /// Appends another SQL fragment to this one.
    ///
    /// Both the SQL text and parameters are merged.
    pub fn append(mut self, other: impl Into<SQL<'a, V>>) -> Self {
        self.chunks.extend(other.into().chunks);
        self
    }

    /// Joins multiple SQL fragments with a separator.
    ///
    /// The separator is inserted between each fragment, but not before the first or after the last.
    pub fn join(sqls: &[impl ToSQL<'a, V>], separator: &'a str) -> SQL<'a, V> {
        if sqls.is_empty() {
            return Self::raw("");
        }

        let mut iter = sqls.into_iter();
        let first = match iter.next() {
            Some(sql) => sql,
            None => return Self::raw(""),
        };

        let mut result = first.to_sql();
        for sql in iter {
            result = result.append_raw(separator);
            result = result.append(sql.to_sql());
        }

        result.clone()
    }

    /// Returns the SQL string represented by this SQL fragment, using placeholders for parameters.
    pub fn sql(&self) -> String {
        let mut result = String::with_capacity(self.chunks.len() * 16); // Estimate capacity
        for (index, chunk) in self.chunks.iter().enumerate() {
            match chunk {
                SQLChunk::Text(text) => result.push_str(text),
                SQLChunk::Param { placeholder, .. } => result.push_str(&placeholder.to_string()),
                SQLChunk::SQL(sql) => result.push_str(&sql.sql()),
            }
            // Add space separator between chunks, except for the last one
            if index < self.chunks.len() - 1 {
                // Check if current or next chunk is Text("(") or Text(")") to avoid extra space
                let next_chunk_is_paren = if index + 1 < self.chunks.len() {
                    matches!(
                        self.chunks.get(index + 1),
                        Some(SQLChunk::Text(Cow::Borrowed(")") | Cow::Borrowed("(")))
                    )
                } else {
                    false
                };
                let current_chunk_is_paren = matches!(
                    chunk,
                    SQLChunk::Text(Cow::Borrowed("(") | Cow::Borrowed(")"))
                );

                if !current_chunk_is_paren && !next_chunk_is_paren {
                    result.push(' ');
                }
            }
        }

        result // No trim needed if spaces are handled correctly
    }

    /// Returns the parameter values from this SQL fragment in the correct order.
    pub fn params(&self) -> Vec<V> {
        let mut params_vec = Vec::new();
        for chunk in &self.chunks {
            match chunk {
                SQLChunk::Param { value, .. } => {
                    params_vec.push(value.clone().into_owned());
                }
                SQLChunk::SQL(sql) => {
                    params_vec.extend(sql.params());
                }
                SQLChunk::Text(_) => { /* Ignore text chunks */ }
            }
        }
        params_vec
    }

    pub fn as_(self, alias: &str) -> SQL<'a, V> {
        self.append_raw(format!("AS {}", alias))
    }
}

impl<'a, V: SQLParam + 'a> From<&'a str> for SQL<'a, V> {
    fn from(s: &'a str) -> Self {
        SQL::raw(s)
    }
}

// Add implementation for references to SQL
impl<'a, 'b, V: SQLParam + 'a> From<&'b SQL<'a, V>> for SQL<'a, V>
where
    'b: 'a,
{
    fn from(sql: &'b SQL<'a, V>) -> Self {
        sql.clone()
    }
}

pub trait ToSQL<'a, V: SQLParam> {
    fn to_sql(&self) -> SQL<'a, V>;
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for () {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::raw("*")
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for SQL<'a, V> {
    fn to_sql(&self) -> SQL<'a, V> {
        self.clone()
    }
}

impl<'a, V: SQLParam + 'a> AsRef<SQL<'a, V>> for SQL<'a, V> {
    fn as_ref(&self) -> &SQL<'a, V> {
        self
    }
}

impl<'a, V: SQLParam + std::fmt::Display> Display for SQL<'a, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.sql())
    }
}

// Implement ToSQL for primitive types
impl<'a, V> ToSQL<'a, V> for &'a str
where
    V: SQLParam + 'a,
    V: From<&'a str>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(Cow::Owned(V::from(*self)))
    }
}

impl<'a, V> ToSQL<'a, V> for String
where
    V: SQLParam + 'a,
    V: From<String>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(Cow::Owned(V::from(self.clone())))
    }
}

impl<'a, V> ToSQL<'a, V> for i32
where
    V: SQLParam + 'a + From<i64>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(Cow::Owned(V::from(*self as i64)))
    }
}

impl<'a, V> ToSQL<'a, V> for i64
where
    V: SQLParam + 'a + From<i64>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(Cow::Owned(V::from(*self)))
    }
}

impl<'a, V> ToSQL<'a, V> for f64
where
    V: SQLParam + 'a + From<f64>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(Cow::Owned(V::from(*self)))
    }
}

impl<'a, V> ToSQL<'a, V> for bool
where
    V: SQLParam + 'a + From<i64>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(Cow::Owned(V::from(*self as i64)))
    }
}

impl<'a, V, T> ToSQL<'a, V> for Option<T>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        match self {
            Some(value) => value.to_sql(), // Let the inner type handle parameterization
            None => SQL::raw("NULL"),      // NULL is a keyword, use raw
        }
    }
}

#[cfg(feature = "uuid")]
impl<'a, V> ToSQL<'a, V> for Uuid
where
    V: SQLParam + 'a,
    V: From<Uuid>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(Cow::Owned(V::from(*self)))
    }
}
pub mod placeholders {
    use super::{Placeholder, PlaceholderStyle};

    pub fn colon(name: &'static str) -> Placeholder {
        Placeholder::with_style(name, PlaceholderStyle::Colon)
    }

    pub fn at(name: &'static str) -> Placeholder {
        Placeholder::with_style(name, PlaceholderStyle::AtSign)
    }

    pub fn dollar(name: &'static str) -> Placeholder {
        Placeholder::with_style(name, PlaceholderStyle::Dollar)
    }
}

#[macro_export]
macro_rules! columns {
    () => {
        ["*".to_sql()]
    };
    ($($column:expr),+ $(,)?) => {
        [$($column.to_sql()),+]
    };
}
