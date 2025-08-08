pub mod error;
pub mod expressions;
pub mod helpers;
pub mod traits;

use smallvec::{SmallVec, smallvec};
use std::{borrow::Cow, fmt, fmt::Display};
use traits::SQLParam;
// Re-export key traits from traits module
pub use traits::{
    IsInSchema, SQLColumn, SQLColumnInfo, SQLComparable, SQLPartial, SQLSchema, SQLTable,
    SQLTableInfo,
};

#[cfg(feature = "uuid")]
use uuid::Uuid;

// TODO: Figure out the best way to incorp dialect in our types?
/// Represents a SQL dialect
pub enum Dialect {
    SQLite,
    PostgreSQL,
    MySQL,
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

/// Sort direction for ORDER BY clauses
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderBy {
    Asc,
    Desc,
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for OrderBy {
    fn to_sql(&self) -> SQL<'a, V> {
        let sql_str = match self {
            OrderBy::Asc => "ASC",
            OrderBy::Desc => "DESC",
        };
        SQL::raw(sql_str)
    }
}

/// Various styles of SQL parameter placeholders.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Default, Debug, Clone, Hash, Copy, PartialEq, Eq)]
pub struct Placeholder<'a> {
    /// The name of the parameter.
    pub name: Option<&'a str>,
    /// The style of the placeholder.
    pub style: PlaceholderStyle,
}

impl<'a> Placeholder<'a> {
    /// Creates a new placeholder with the given name and style.
    pub const fn with_style(name: &'a str, style: PlaceholderStyle) -> Self {
        Placeholder {
            name: Some(name),
            style,
        }
    }

    /// Creates a new colon-style placeholder.
    pub const fn colon(name: &'a str) -> Self {
        Self::with_style(name, PlaceholderStyle::Colon)
    }

    /// Creates a new at-sign-style placeholder.
    pub const fn at(name: &'a str) -> Self {
        Self::with_style(name, PlaceholderStyle::AtSign)
    }

    /// Creates a new dollar-style placeholder.
    pub const fn dollar(name: &'a str) -> Self {
        Self::with_style(name, PlaceholderStyle::Dollar)
    }

    /// Creates a positional placeholder ('?').
    pub const fn positional() -> Self {
        Placeholder {
            name: None,
            style: PlaceholderStyle::Positional,
        }
    }
}

impl<'a> fmt::Display for Placeholder<'a> {
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
        placeholder: Cow<'a, Placeholder<'a>>,
    },
    SQL(Box<SQL<'a, V>>),
    /// A table reference that can render itself with proper schema/alias handling
    Table(&'a dyn SQLTableInfo),
    /// A column reference that can render itself with proper table qualification
    Column(&'a dyn SQLColumnInfo),
    /// An alias wrapping any SQL chunk: "chunk AS alias"
    Alias {
        chunk: Box<SQLChunk<'a, V>>,
        alias: Cow<'a, str>,
    },
    /// A subquery wrapped in parentheses: "(SELECT ...)"
    Subquery(Box<SQL<'a, V>>),
}

impl<'a, V: SQLParam + 'a> SQLChunk<'a, V> {
    /// Creates a text chunk from a borrowed string - zero allocation
    pub const fn text(text: &'a str) -> Self {
        Self::Text(Cow::Borrowed(text))
    }

    /// Creates a parameter chunk with borrowed value and placeholder
    pub const fn param(value: &'a V, placeholder: &'a Placeholder) -> Self {
        Self::Param {
            value: Cow::Borrowed(value),
            placeholder: Cow::Borrowed(placeholder),
        }
    }

    /// Creates a nested SQL chunk
    pub fn sql(sql: SQL<'a, V>) -> Self {
        Self::SQL(Box::new(sql))
    }

    /// Creates a table chunk
    pub fn table(table: &'a dyn SQLTableInfo) -> Self {
        Self::Table(table)
    }

    /// Creates a column chunk
    pub fn column(column: &'a dyn SQLColumnInfo) -> Self {
        Self::Column(column)
    }

    /// Creates an alias chunk wrapping any SQLChunk
    pub fn alias(chunk: SQLChunk<'a, V>, alias: impl Into<Cow<'a, str>>) -> Self {
        Self::Alias {
            chunk: Box::new(chunk),
            alias: alias.into(),
        }
    }

    /// Creates a subquery chunk
    pub fn subquery(sql: SQL<'a, V>) -> Self {
        Self::Subquery(Box::new(sql))
    }

    /// Renders a single SQL chunk to a string
    fn render(&self) -> String {
        match self {
            SQLChunk::Text(text) => text.to_string(),
            SQLChunk::Param { placeholder, .. } => placeholder.to_string(),
            SQLChunk::SQL(sql) => sql.sql(),
            SQLChunk::Table(table) => table.name().to_string(),
            SQLChunk::Column(column) => {
                format!(r#""{}"."{}""#, column.table().name(), column.name())
            }
            SQLChunk::Alias { chunk, alias } => {
                format!("{} AS {}", Self::render(chunk), alias)
            }

            SQLChunk::Subquery(sql) => format!("({})", sql.sql()),
        }
    }
}

/// A SQL statement or fragment with parameters.
///
/// This type is used to build SQL statements with proper parameter handling.
/// It keeps track of both the SQL text and the parameters to be bound.
#[derive(Debug, Clone)]
pub struct SQL<'a, V: SQLParam + 'a> {
    /// The chunks that make up this SQL statement or fragment.
    pub chunks: SmallVec<[SQLChunk<'a, V>; 3]>,
}

impl<'a, V: SQLParam + 'a> SQL<'a, V> {
    /// Const placeholder instances for zero-copy usage
    const POSITIONAL_PLACEHOLDER: Placeholder<'a> = Placeholder::positional();

    pub const fn new<'b>(sql: &'b str) -> SQL<'a, V> {
        unimplemented!()
    }

    /// Creates a new empty SQL fragment.
    pub const fn empty<'b>() -> SQL<'b, V> {
        SQL {
            chunks: SmallVec::new_const(),
        }
    }

    /// Helper to create const SQL
    const fn text(text: &'a str) -> Self {
        // Create a SmallVec with the single chunk and pad with empty chunks
        let chunks = SmallVec::from_const([
            SQLChunk::Text(Cow::Borrowed(text)),
            SQLChunk::Text(Cow::Borrowed("")), // These will be ignored in sql() generation
            SQLChunk::Text(Cow::Borrowed("")),
        ]);
        Self { chunks }
    }

    /// Creates a wildcard SELECT fragment: "*"
    pub const fn wildcard() -> Self {
        Self::text("*")
    }

    /// Creates a NULL SQL fragment
    pub const fn null() -> Self {
        Self::text("NULL")
    }

    /// Creates a TRUE SQL fragment
    pub const fn r#true() -> Self {
        Self::text("TRUE")
    }

    /// Creates a FALSE SQL fragment
    pub const fn r#false() -> Self {
        Self::text("FALSE")
    }

    /// Creates a new SQL fragment from a raw string.
    ///
    /// The string is treated as literal SQL text, not a parameter.
    pub fn raw<T: Into<Cow<'a, str>>>(sql: T) -> Self {
        Self {
            chunks: smallvec![SQLChunk::Text(sql.into())],
        }
    }

    /// Creates a new SQL fragment representing a parameter.
    ///
    /// A default positional placeholder ('?') is used, and the provided value
    /// is stored for later binding. Accepts both owned and borrowed values.
    pub fn parameter(param: impl Into<Cow<'a, V>>) -> Self {
        Self {
            chunks: smallvec![SQLChunk::Param {
                value: param.into(),
                placeholder: Cow::Borrowed(&Self::POSITIONAL_PLACEHOLDER),
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
    /// Both the SQL text and parameters are merged. Flattens nested SQL chunks.
    pub fn append(mut self, other: impl Into<SQL<'a, V>>) -> Self {
        let other_sql = other.into();

        // Flatten nested SQL chunks to avoid deep nesting
        for chunk in other_sql.chunks {
            match chunk {
                SQLChunk::SQL(nested_sql) => {
                    self.chunks.extend(nested_sql.chunks);
                }
                chunk => self.chunks.push(chunk),
            }
        }
        self
    }

    /// Pre-allocates capacity for additional chunks.
    pub fn with_capacity(mut self, additional: usize) -> Self {
        self.chunks.reserve(additional);
        self
    }

    /// Joins multiple SQL fragments with a separator.
    ///
    /// The separator is inserted between each fragment, but not before the first or after the last.
    pub fn join<T>(sqls: T, separator: &'a str) -> SQL<'a, V>
    where
        T: IntoIterator,
        T::Item: ToSQL<'a, V>,
    {
        let mut chunks = SmallVec::new();
        let mut first = true;

        for sql in sqls {
            if !first {
                chunks.push(SQLChunk::Text(Cow::Borrowed(separator)));
            }
            first = false;

            // Flatten nested SQL chunks to avoid deep nesting
            let sql_chunks = sql.to_sql().chunks;
            chunks.extend(sql_chunks);
        }

        SQL { chunks }
    }

    /// Collects parameter values from a single chunk
    fn collect_chunk_params(chunk: &SQLChunk<'a, V>) -> Vec<V> {
        match chunk {
            SQLChunk::Param { value, .. } => vec![value.as_ref().clone()],
            SQLChunk::SQL(sql) => sql.params(),
            SQLChunk::Alias { chunk, .. } => Self::collect_chunk_params(chunk),
            SQLChunk::Subquery(sql) => sql.params(),
            _ => vec![],
        }
    }

    /// Returns the SQL string represented by this SQL fragment, using placeholders for parameters.
    pub fn sql(&self) -> String {
        if self.chunks.is_empty() {
            return String::new();
        }
        if self.chunks.len() == 1 {
            return self.chunks[0].render();
        }

        // Better capacity estimation based on chunk content
        let estimated_size = self
            .chunks
            .iter()
            .map(|chunk| match chunk {
                SQLChunk::Text(text) => text.len(),
                SQLChunk::Param { .. } => 1, // "?" placeholder
                SQLChunk::SQL(sql) => sql.chunks.len() * 8, // Rough estimate
                SQLChunk::Table(table) => table.name().len(),
                SQLChunk::Column(column) => column.name().len(),
                SQLChunk::Alias { chunk: _, alias } => alias.len() + 4, // " AS " + alias
                SQLChunk::Subquery(sql) => sql.chunks.len() * 8 + 2,    // Rough estimate + "()"
            })
            .sum::<usize>()
            + self.chunks.len(); // +1 space per chunk

        let mut result = String::with_capacity(estimated_size);

        for (index, chunk) in self.chunks.iter().enumerate() {
            match chunk {
                SQLChunk::Text(text) => {
                    if !text.is_empty() {
                        result.push_str(text);
                    } else {
                        continue; // Skip empty padding chunks
                    }
                }
                SQLChunk::Param { placeholder, .. } => result.push_str(&placeholder.to_string()),
                SQLChunk::SQL(sql) => result.push_str(&sql.sql()),
                SQLChunk::Table(table) => {
                    result.push('\"');
                    result.push_str(table.name());
                    result.push('\"');
                }
                SQLChunk::Column(column) => {
                    result.push('\"');
                    result.push_str(column.table().name());
                    result.push('\"');
                    result.push('.');
                    result.push('\"');
                    result.push_str(column.name());
                    result.push('\"');
                }
                SQLChunk::Alias { chunk, alias } => {
                    result.push_str(&chunk.render());
                    result.push_str(" AS ");
                    result.push_str(alias);
                }
                SQLChunk::Subquery(sql) => {
                    result.push('(');
                    result.push_str(&sql.sql());
                    result.push(')');
                }
            }

            // Add space separator between chunks, except for the last one
            if index < self.chunks.len() - 1 {
                // Check if current or next chunk needs spacing
                let current_needs_space = !matches!(
                    chunk,
                    SQLChunk::Text(text) if text.ends_with('(') || text.ends_with(',') || text.ends_with(' ')
                );
                let next_needs_space = !matches!(
                    self.chunks.get(index + 1),
                    Some(SQLChunk::Text(text)) if text.starts_with(')') || text.starts_with(',') || text.starts_with(' ')
                );

                if current_needs_space && next_needs_space {
                    result.push(' ');
                }
            }
        }

        result
    }

    /// Returns the parameter values from this SQL fragment in the correct order.
    pub fn params(&self) -> Vec<V> {
        // Pre-allocate based on rough estimate
        let mut params_vec = Vec::with_capacity(self.chunks.len().min(8));

        for chunk in &self.chunks {
            params_vec.extend(Self::collect_chunk_params(chunk));
        }
        params_vec
    }

    /// Returns references to parameter values - zero-copy when possible.
    pub fn param_refs(&'a self) -> Vec<&'a V> {
        let mut param_refs = Vec::with_capacity(self.chunks.len().min(8));

        fn collect_refs<'a, V: SQLParam + 'a>(chunk: &'a SQLChunk<'a, V>, refs: &mut Vec<&'a V>) {
            match chunk {
                SQLChunk::Param { value, .. } => {
                    refs.push(value.as_ref());
                }
                SQLChunk::SQL(sql) => {
                    refs.extend(sql.param_refs());
                }
                SQLChunk::Alias { chunk, .. } => {
                    collect_refs(chunk, refs);
                }
                SQLChunk::Subquery(sql) => {
                    refs.extend(sql.param_refs());
                }
                _ => {}
            }
        }

        for chunk in &self.chunks {
            collect_refs(chunk, &mut param_refs);
        }
        param_refs
    }

    pub fn as_(self, alias: &str) -> SQL<'a, V> {
        self.append_raw(format!("AS {}", alias))
    }

    /// Creates an aliased version of this SQL using the Alias chunk (more context-aware)
    pub fn alias(self, alias: impl Into<Cow<'a, str>>) -> SQL<'a, V> {
        SQL {
            chunks: smallvec![SQLChunk::Alias {
                chunk: Box::new(SQLChunk::SQL(Box::new(self))),
                alias: alias.into(),
            }],
        }
    }

    /// Wraps this SQL as a subquery
    pub fn subquery(self) -> SQL<'a, V> {
        SQL {
            chunks: smallvec![SQLChunk::subquery(self)],
        }
    }

    /// Creates a comma-separated list of column names: "col1, col2, col3"
    pub fn columns(names: &[&'a str]) -> Self {
        if names.is_empty() {
            return Self::raw("*");
        }
        if names.len() == 1 {
            return Self::raw(names[0]);
        }

        // Direct chunk building: name, ", ", name, ", ", name
        let mut chunks = SmallVec::with_capacity(names.len() * 2 - 1);
        for (i, name) in names.iter().enumerate() {
            if i > 0 {
                chunks.push(SQLChunk::Text(Cow::Borrowed(", ")));
            }
            chunks.push(SQLChunk::Text(Cow::Borrowed(name)));
        }
        SQL { chunks }
    }

    /// Creates a comma-separated list of parameter placeholders with values: "?, ?, ?"
    /// Accepts both owned Vec and borrowed slices for zero-copy when possible.
    pub fn parameters<I>(values: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Cow<'a, V>>,
    {
        let values: Vec<_> = values.into_iter().collect();
        if values.is_empty() {
            return Self::empty();
        }
        if values.len() == 1 {
            return Self::parameter(values.into_iter().next().unwrap());
        }

        // Direct chunk building: param, ", ", param, ", ", param
        let mut chunks = SmallVec::with_capacity(values.len() * 2 - 1);
        for (i, value) in values.into_iter().enumerate() {
            if i > 0 {
                chunks.push(SQLChunk::Text(Cow::Borrowed(", ")));
            }
            chunks.push(SQLChunk::Param {
                value: value.into(),
                placeholder: Cow::Borrowed(&Self::POSITIONAL_PLACEHOLDER),
            });
        }
        SQL { chunks }
    }

    /// Creates a comma-separated list of column assignments: "col1 = ?, col2 = ?"
    /// Accepts both owned and borrowed iterators for zero-copy when possible.
    pub fn assignments<I, T>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (&'a str, T)>,
        T: Into<Cow<'a, V>>,
    {
        let pairs: Vec<_> = pairs.into_iter().collect();
        if pairs.is_empty() {
            return Self::empty();
        }
        if pairs.len() == 1 {
            let (col, val) = pairs.into_iter().next().unwrap();
            return Self::raw(col)
                .append_raw(" = ")
                .append(Self::parameter(val.into()));
        }

        // Direct chunk building: col, " = ", param, ", ", col, " = ", param
        let mut chunks = SmallVec::with_capacity(pairs.len() * 4 - 1);
        for (i, (col, val)) in pairs.into_iter().enumerate() {
            if i > 0 {
                chunks.push(SQLChunk::Text(Cow::Borrowed(", ")));
            }
            chunks.push(SQLChunk::Text(Cow::Borrowed(col)));
            chunks.push(SQLChunk::Text(Cow::Borrowed(" = ")));
            chunks.push(SQLChunk::Param {
                value: val.into(),
                placeholder: Cow::Borrowed(&Self::POSITIONAL_PLACEHOLDER),
            });
        }
        SQL { chunks }
    }
}

impl<'a, V: SQLParam + 'a> Default for SQL<'a, V> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<'a, V: SQLParam + 'a> From<&'a str> for SQL<'a, V> {
    fn from(s: &'a str) -> Self {
        SQL::raw(s)
    }
}

// // Add implementation for references to SQL
// impl<'a, 'b, V: SQLParam + 'a> From<&'b SQL<'a, V>> for SQL<'a, V>
// where
//     'b: 'a,
// {
//     fn from(sql: &'b SQL<'a, V>) -> Self {
//         sql.clone()
//     }
// }

pub trait ToSQL<'a, V: SQLParam> {
    fn to_sql(&self) -> SQL<'a, V>;
}

impl<'a, T, V> From<&T> for SQL<'a, V>
where
    T: ToSQL<'a, V>,
    V: SQLParam,
{
    fn from(value: &T) -> Self {
        value.to_sql()
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for () {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::from("*")
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

impl<'a, V, T> ToSQL<'a, V> for Vec<T>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::join(self.iter().map(ToSQL::to_sql), ", ")
    }
}

impl<'a, V, T> ToSQL<'a, V> for &'a [T]
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::join(self.iter().map(ToSQL::to_sql), ", ")
    }
}

impl<'a, V, T, const N: usize> ToSQL<'a, V> for [T; N]
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::join(self.iter().map(ToSQL::to_sql), ", ")
    }
}

// Implement ToSQL for SQLTableInfo and SQLColumnInfo trait objects
impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for &'a dyn SQLTableInfo {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL {
            chunks: smallvec![SQLChunk::table(*self)],
        }
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for &'a dyn SQLColumnInfo {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL {
            chunks: smallvec![SQLChunk::column(*self)],
        }
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

    pub const fn colon<'a>(name: &'a str) -> Placeholder<'a> {
        Placeholder::with_style(name, PlaceholderStyle::Colon)
    }

    pub const fn at<'a>(name: &'a str) -> Placeholder<'a> {
        Placeholder::with_style(name, PlaceholderStyle::AtSign)
    }

    pub const fn dollar<'a>(name: &'a str) -> Placeholder<'a> {
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

#[macro_export]
macro_rules! sql {
    // String template pattern: sql!("SELECT {} FROM {}", col, table)
    ($template:literal, $($arg:expr),+ $(,)?) => {
        {
            let template_str = $template;
            let mut parts = template_str.split("{}");
            let args = vec![$($arg.to_sql()),+];

            let mut result = $crate::SQL::raw(parts.next().unwrap_or(""));
            for (i, part) in parts.enumerate() {
                if i < args.len() {
                    result = result.append(args[i].clone());
                }
                if !part.is_empty() {
                    result = result.append_raw(part);
                }
            }
            result
        }
    };

    // Tuple array pattern: sql!([(col1, OrderBy::Asc), (col2, OrderBy::Desc)])
    ($(($first:expr, $sec:expr)),*) => {
        {
            [$(($first.to_sql(), $sec)),*]
        }
    };

    // Array pattern: sql!([col1, col2, col3]) -> [col1.to_sql(), col2.to_sql(), col3.to_sql()]
    ([$($item:expr),+ $(,)?]) => {
        [$($item.to_sql()),+]
    };
    // Single value pattern: sql!("hello") -> SQL::parameter("hello")
    ($value:literal) => {
       $value.to_sql()
    };
}

/// A generic alias wrapper for tables that maintains type information through PhantomData
#[derive(Debug, Clone, Copy, Default)]
pub struct Alias<T = ()> {
    name: &'static str,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Alias<T> {
    /// Creates a new table alias instance
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T> traits::SQLAlias for Alias<T> {
    fn alias(&self) -> &'static str {
        self.name
    }
}

impl<'a, T, V> ToSQL<'a, V> for Alias<T>
where
    T: traits::SQLTable<'a, V>,
    V: traits::SQLParam + 'a,
{
    fn to_sql(&self) -> SQL<'a, V> {
        let table = T::default();
        let static_table: &'a dyn traits::SQLTableInfo =
            unsafe { std::mem::transmute(&table as &dyn traits::SQLTableInfo) };
        SQL {
            chunks: smallvec![SQLChunk::Alias {
                chunk: Box::new(SQLChunk::table(static_table)),
                alias: std::borrow::Cow::Borrowed(self.name),
            }],
        }
    }
}

/// Creates an aliased table that can be used in joins and queries
/// Usage: alias!(User, "u") creates an alias of the User table with alias "u"
#[macro_export]
macro_rules! alias {
    ($table:ty, $alias_name:literal) => {
        $crate::Alias::<$table>::new($alias_name)
    };
}
