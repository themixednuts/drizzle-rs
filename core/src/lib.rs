pub mod error;
pub mod expressions;
pub mod helpers;
pub mod prepared;
pub mod traits;

use compact_str::{CompactString, ToCompactString};
use smallvec::{SmallVec, smallvec};
use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::{self, Display},
};
// Re-export key traits from traits module
pub use traits::*;

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
pub struct Placeholder {
    /// The name of the parameter.
    pub name: Option<&'static str>,
    /// The style of the placeholder.
    pub style: PlaceholderStyle,
}

impl Placeholder {
    /// Creates a new placeholder with the given name and style.
    pub const fn with_style(name: &'static str, style: PlaceholderStyle) -> Self {
        Placeholder {
            name: Some(name),
            style,
        }
    }

    /// Creates a new colon-style placeholder.
    pub const fn colon(name: &'static str) -> Self {
        Self::with_style(name, PlaceholderStyle::Colon)
    }

    /// Creates a new at-sign-style placeholder.
    pub const fn at(name: &'static str) -> Self {
        Self::with_style(name, PlaceholderStyle::AtSign)
    }

    /// Creates a new dollar-style placeholder.
    pub const fn dollar(name: &'static str) -> Self {
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

/// A SQL parameter that associates a value with a placeholder.
/// Designed to be const-friendly and zero-cost when possible.
#[derive(Debug, Clone)]
pub struct Param<'a, V: SQLParam> {
    /// The placeholder to use in the SQL
    pub placeholder: Placeholder,
    /// The value to bind
    pub value: Option<Cow<'a, V>>,
}

impl<'a, V: SQLParam> Param<'a, V> {
    pub fn new(placeholder: Placeholder, value: Option<Cow<'a, V>>) -> Self {
        Self { placeholder, value }
    }
}

impl<'a, V: SQLParam> From<OwnedParam<V>> for Param<'a, V> {
    fn from(value: OwnedParam<V>) -> Self {
        Self {
            placeholder: value.placeholder,
            value: value.value.map(|v| Cow::Owned(v)),
        }
    }
}

impl<'a, V: SQLParam> From<&'a OwnedParam<V>> for Param<'a, V> {
    fn from(value: &'a OwnedParam<V>) -> Self {
        Self {
            placeholder: value.placeholder,
            value: value.value.as_ref().map(|v| Cow::Borrowed(v)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OwnedParam<V: SQLParam> {
    /// The placeholder to use in the SQL
    pub placeholder: Placeholder,
    /// The value to bind
    pub value: Option<V>,
}

impl<'a, V: SQLParam> From<Param<'a, V>> for OwnedParam<V> {
    fn from(value: Param<'a, V>) -> Self {
        Self {
            placeholder: value.placeholder,
            value: value.value.map(|v| v.into_owned()),
        }
    }
}

impl<'a, V: SQLParam> From<&Param<'a, V>> for OwnedParam<V> {
    fn from(value: &Param<'a, V>) -> Self {
        Self {
            placeholder: value.placeholder,
            value: value.value.clone().map(|v| v.into_owned()),
        }
    }
}

impl<'a, T: SQLParam> Param<'a, T> {
    /// Creates a new parameter with a positional placeholder
    pub const fn positional(value: T) -> Self {
        Self {
            placeholder: Placeholder::positional(),
            value: Some(Cow::Owned(value)),
        }
    }

    /// Creates a new parameter with a specific placeholder and no value
    pub const fn from_placeholder(placeholder: Placeholder) -> Self {
        Self {
            placeholder,
            value: None,
        }
    }

    /// Creates a new parameter with a named placeholder (colon style)
    pub const fn named(name: &'static str, value: T) -> Self {
        Self {
            placeholder: Placeholder::colon(name),
            value: Some(Cow::Owned(value)),
        }
    }

    /// Creates a new parameter with a specific placeholder
    pub const fn with_placeholder(placeholder: Placeholder, value: T) -> Self {
        Self {
            placeholder,
            value: Some(Cow::Owned(value)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParamBind<'a, V: SQLParam> {
    pub name: &'a str,
    pub value: V,
}

impl<'a, V: SQLParam> ParamBind<'a, V> {
    pub const fn new(name: &'a str, value: V) -> Self {
        Self { name, value }
    }
}

/// A SQL chunk represents a part of an SQL statement.
#[derive(Clone)]
pub enum SQLChunk<'a, V: SQLParam + 'a> {
    Text(Cow<'a, CompactString>),
    Param(Param<'a, V>),
    SQL(Box<SQL<'a, V>>),
    /// A table reference that can render itself with proper schema/alias handling
    Table(&'a dyn SQLTableInfo),
    /// A column reference that can render itself with proper table qualification
    Column(&'a dyn SQLColumnInfo),
    /// An alias wrapping any SQL chunk: "chunk AS alias"
    Alias {
        chunk: Box<SQLChunk<'a, V>>,
        alias: CompactString,
    },
    /// A subquery wrapped in parentheses: "(SELECT ...)"
    Subquery(Box<SQL<'a, V>>),
}

impl<'a, V: SQLParam + 'a> SQLChunk<'a, V> {
    /// Creates a text chunk from a borrowed string - zero allocation
    pub const fn text(text: &'static str) -> Self {
        Self::Text(Cow::Owned(CompactString::const_new(text)))
    }

    /// Creates a parameter chunk with borrowed value and placeholder
    pub const fn param(value: &'a V, placeholder: Placeholder) -> Self {
        Self::Param(Param {
            value: Some(Cow::Borrowed(value)),
            placeholder: placeholder,
        })
    }

    /// Creates a nested SQL chunk
    pub fn sql(sql: SQL<'a, V>) -> Self {
        Self::SQL(Box::new(sql))
    }

    /// Creates a table chunk
    pub const fn table(table: &'a dyn SQLTableInfo) -> Self {
        Self::Table(table)
    }

    /// Creates a column chunk
    pub const fn column(column: &'a dyn SQLColumnInfo) -> Self {
        Self::Column(column)
    }

    /// Creates an alias chunk wrapping any SQLChunk
    pub fn alias(chunk: SQLChunk<'a, V>, alias: impl Into<CompactString>) -> Self {
        Self::Alias {
            chunk: Box::new(chunk),
            alias: alias.into(),
        }
    }

    /// Creates a subquery chunk
    pub fn subquery(sql: SQL<'a, V>) -> Self {
        Self::Subquery(Box::new(sql))
    }

    /// Write chunk to buffer (zero-allocation internal method)
    fn write_to_buffer(&self, buf: &mut CompactString) {
        match self {
            SQLChunk::Text(text) => buf.push_str(text),
            SQLChunk::Param(Param { placeholder, .. }) => buf.push_str(&placeholder.to_string()),
            SQLChunk::SQL(sql) => buf.push_str(&sql.sql()),
            SQLChunk::Table(table) => {
                buf.push('"');
                buf.push_str(table.name());
                buf.push('"');
            }
            SQLChunk::Column(column) => {
                buf.push('"');
                buf.push_str(column.table().name());
                buf.push_str(r#"".""#);
                buf.push_str(column.name());
                buf.push('"');
            }
            SQLChunk::Alias { chunk, alias } => {
                chunk.write_to_buffer(buf);
                buf.push_str(" AS ");
                buf.push_str(alias);
            }
            SQLChunk::Subquery(sql) => {
                buf.push('(');
                buf.push_str(&sql.sql());
                buf.push(')');
            }
        }
    }
}

impl<'a, V: SQLParam + std::fmt::Debug> std::fmt::Debug for SQLChunk<'a, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SQLChunk::Text(text) => f.debug_tuple("Text").field(text).finish(),
            SQLChunk::Param(param) => f.debug_tuple("Param").field(param).finish(),
            SQLChunk::SQL(_) => f.debug_tuple("SQL").field(&"<nested>").finish(),
            SQLChunk::Table(table) => f.debug_tuple("Table").field(&table.name()).finish(),
            SQLChunk::Column(column) => f
                .debug_tuple("Column")
                .field(&format!("{}.{}", column.table().name(), column.name()))
                .finish(),
            SQLChunk::Alias { alias, .. } => f
                .debug_struct("Alias")
                .field("alias", alias)
                .field("chunk", &"<nested>")
                .finish(),
            SQLChunk::Subquery(_) => f.debug_tuple("Subquery").field(&"<nested>").finish(),
        }
    }
}

/// A SQL statement or fragment with parameters.
///
/// This type is used to build SQL statements with proper parameter handling.
/// It keeps track of both the SQL text and the parameters to be bound.
#[derive(Clone)]
pub struct SQL<'a, V: SQLParam> {
    /// The chunks that make up this SQL statement or fragment.
    pub chunks: SmallVec<[SQLChunk<'a, V>; 3]>,
}

impl<'a, V: SQLParam + 'a> SQL<'a, V> {
    /// Const placeholder instances for zero-copy usage
    const POSITIONAL_PLACEHOLDER: Placeholder = Placeholder::positional();

    pub const fn new<'b>(chunks: [SQLChunk<'a, V>; 3]) -> SQL<'a, V> {
        SQL {
            chunks: SmallVec::from_const(chunks),
        }
    }

    /// Creates a new empty SQL fragment.
    pub const fn empty<'b>() -> SQL<'b, V> {
        SQL {
            chunks: SmallVec::new_const(),
        }
    }

    /// Helper to create const SQL
    pub const fn text(text: &'static str) -> Self {
        // Create a SmallVec with the single chunk and pad with empty chunks
        let chunks = SmallVec::from_const([
            SQLChunk::Text(Cow::Owned(CompactString::const_new(text))),
            SQLChunk::Text(Cow::Owned(CompactString::const_new(""))), // These will be ignored in sql() generation
            SQLChunk::Text(Cow::Owned(CompactString::const_new(""))),
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
    pub fn raw<T: AsRef<str>>(sql: T) -> Self {
        let sql = Cow::Owned(sql.as_ref().try_to_compact_string().unwrap_or_default());

        Self {
            chunks: smallvec![SQLChunk::Text(sql)],
        }
    }

    /// Creates a new SQL fragment representing a parameter.
    ///
    /// A default positional placeholder ('?') is used, and the provided value
    /// is stored for later binding. Accepts both owned and borrowed values.
    pub fn parameter(param: impl Into<Cow<'a, V>>) -> Self {
        Self {
            chunks: smallvec![SQLChunk::Param(Param {
                value: Some(param.into()),
                placeholder: Self::POSITIONAL_PLACEHOLDER,
            })],
        }
    }

    /// Creates a new SQL fragment representing a table.
    pub fn table<'b>(table: &'b dyn SQLTableInfo) -> SQL<'b, V> {
        SQL {
            chunks: smallvec![SQLChunk::table(table)],
        }
    }

    pub fn column<'b>(column: &'b dyn SQLColumnInfo) -> SQL<'b, V> {
        SQL {
            chunks: smallvec![SQLChunk::column(column)],
        }
    }

    /// Creates a named placeholder without a value - for use in query building.
    /// Similar to drizzle-orm's sql.placeholder('name').
    /// The value will be bound later during execution.
    pub fn placeholder(name: &'static str) -> Self
    where
        V: Default,
    {
        Self {
            chunks: smallvec![SQLChunk::Param(Param {
                value: None,
                placeholder: Placeholder::colon(name),
            })],
        }
    }

    /// Creates a named placeholder with a specific style.
    pub fn placeholder_with_style(name: &'static str, style: PlaceholderStyle) -> Self
    where
        V: Default,
    {
        Self {
            chunks: smallvec![SQLChunk::Param(Param {
                value: None, // Temporary default value
                placeholder: Placeholder::with_style(name, style),
            })],
        }
    }

    /// Creates SQL from an existing Placeholder struct.
    pub fn from_placeholder(placeholder: Placeholder) -> Self
    where
        V: Default,
    {
        Self {
            chunks: smallvec![SQLChunk::Param(Param::from_placeholder(placeholder))],
        }
    }

    /// Appends a raw string to this SQL fragment.
    ///
    /// The string is treated as literal SQL text, not a parameter.
    pub fn append_raw(self, sql: impl AsRef<str>) -> Self {
        self.append(SQL::raw(sql))
    }

    /// Appends another SQL fragment to this one.
    ///
    /// Both the SQL text and parameters are merged.
    pub fn append(mut self, other: impl Into<SQL<'a, V>>) -> Self {
        let other_sql = other.into();
        self.chunks.extend(other_sql.chunks);
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
    pub fn join<T>(sqls: T, separator: &'static str) -> SQL<'a, V>
    where
        T: IntoIterator,
        T::Item: ToSQL<'a, V>,
    {
        let mut chunks = SmallVec::new();
        let mut first = true;

        for sql in sqls {
            if !first {
                chunks.push(SQLChunk::Text(Cow::Owned(CompactString::const_new(
                    separator,
                ))));
            }
            first = false;

            // Flatten nested SQL chunks to avoid deep nesting
            let sql_chunks = sql.to_sql().chunks;
            chunks.extend(sql_chunks);
        }

        SQL { chunks }
    }

    /// Collects parameter references from a single chunk
    fn collect_chunk_params<'b>(chunk: &'b SQLChunk<'a, V>) -> Vec<&'b V> {
        match chunk {
            SQLChunk::Param(Param {
                value: Some(value), ..
            }) => vec![value.as_ref()],
            SQLChunk::SQL(sql) => sql.params(),
            SQLChunk::Alias { chunk, .. } => Self::collect_chunk_params(chunk),
            SQLChunk::Subquery(sql) => sql.params(),
            _ => vec![],
        }
    }

    /// Returns the SQL string represented by this SQL fragment, using placeholders for parameters.
    pub fn sql(&self) -> String {
        match self.chunks.len() {
            0 => String::new(),
            1 => self.render_single_chunk(0).to_string(),
            _ => {
                let capacity = self.estimate_capacity();
                let mut buf = CompactString::with_capacity(capacity);
                self.write_sql(&mut buf);
                buf.into()
            }
        }
    }

    pub fn bind(self, params: impl IntoIterator<Item = ParamBind<'a, V>>) -> SQL<'a, V> {
        let param_map: HashMap<&str, V> = params.into_iter().map(|p| (p.name, p.value)).collect();

        let bound_chunks = self
            .into_iter()
            .map(|chunk| match chunk {
                SQLChunk::Param(mut param) => {
                    // Only bind named placeholders
                    if let Some(name) = param.placeholder.name {
                        if let Some(value) = param_map.get(name) {
                            param.value = Some(Cow::Owned(value.clone()));
                        }
                    }
                    SQLChunk::Param(param)
                }
                other => other,
            })
            .collect();

        SQL {
            chunks: bound_chunks,
        }
    }

    /// Write fully qualified columns to buffer - unified implementation
    fn write_qualified_columns(&self, buf: &mut CompactString, table: &'a dyn SQLTableInfo) {
        let columns = table.columns();
        if columns.is_empty() {
            buf.push('*');
            return;
        }

        for (i, col) in columns.iter().enumerate() {
            if i > 0 {
                buf.push_str(", ");
            }
            buf.push('"');
            buf.push_str(table.name());
            buf.push_str(r#"".""#);
            buf.push_str(col.name());
            buf.push('"');
        }
    }

    /// Zero-allocation SQL writing to existing buffer
    fn write_sql(&self, buf: &mut CompactString) {
        for (i, chunk) in self.chunks.iter().enumerate() {
            self.write_chunk(buf, chunk, i);

            if i + 1 < self.chunks.len() && self.needs_space(chunk, i) {
                buf.push(' ');
            }
        }
    }

    /// Write a single chunk to buffer with pattern detection
    fn write_chunk(&self, buf: &mut CompactString, chunk: &SQLChunk<'a, V>, index: usize) {
        match chunk {
            SQLChunk::Text(text) if text.is_empty() => {
                if let Some(table) = self.detect_pattern_at(index) {
                    self.write_qualified_columns(buf, table);
                }
            }
            SQLChunk::Text(text) if text.trim().eq_ignore_ascii_case("SELECT") => {
                if let Some(table) = self.detect_select_from_table_pattern(index) {
                    buf.push_str("SELECT ");
                    self.write_qualified_columns(buf, table);
                } else {
                    buf.push_str(text);
                }
            }
            SQLChunk::Text(text) => buf.push_str(text),
            SQLChunk::Param(Param { placeholder, .. }) => buf.push_str(&placeholder.to_string()),
            SQLChunk::SQL(sql) => buf.push_str(&sql.sql()),
            SQLChunk::Table(table) => {
                buf.push('"');
                buf.push_str(table.name());
                buf.push('"');
            }
            SQLChunk::Column(column) => {
                buf.push('"');
                buf.push_str(column.table().name());
                buf.push_str(r#"".""#);
                buf.push_str(column.name());
                buf.push('"');
            }
            SQLChunk::Alias { chunk, alias } => {
                chunk.write_to_buffer(buf);
                buf.push_str(" AS ");
                buf.push_str(alias);
            }
            SQLChunk::Subquery(sql) => {
                buf.push('(');
                buf.push_str(&sql.sql());
                buf.push(')');
            }
        }
    }

    /// Fast single chunk rendering - reuses write_chunk for consistency
    fn render_single_chunk(&self, index: usize) -> CompactString {
        let chunk = &self.chunks[index];
        let capacity = match chunk {
            SQLChunk::Text(text) if text.is_empty() => self
                .detect_pattern_at(index)
                .map(|table| table.columns().len() * 20)
                .unwrap_or(0),
            SQLChunk::Text(text) => text.len(),
            SQLChunk::Table(table) => table.name().len() + 2,
            SQLChunk::Column(column) => column.table().name().len() + column.name().len() + 5,
            SQLChunk::Alias { alias, .. } => alias.len() + 10,
            _ => 32,
        };

        let mut buf = CompactString::with_capacity(capacity);
        self.write_chunk(&mut buf, chunk, index);
        buf
    }

    /// Pattern detection: looks for SELECT ... FROM table patterns
    fn detect_pattern_at(&self, empty_index: usize) -> Option<&'a dyn SQLTableInfo> {
        let select_pos = self.find_select_before(empty_index)?;
        self.find_table_after_from(select_pos)
    }

    /// Detect SELECT-FROM-TABLE pattern starting from SELECT index
    fn detect_select_from_table_pattern(
        &self,
        select_index: usize,
    ) -> Option<&'a dyn SQLTableInfo> {
        if select_index + 2 < self.chunks.len() {
            if let (SQLChunk::Text(from_text), SQLChunk::Table(table)) = (
                &self.chunks[select_index + 1],
                &self.chunks[select_index + 2],
            ) {
                if from_text.trim().eq_ignore_ascii_case("FROM") {
                    return Some(*table);
                }
            }
        }
        None
    }

    /// Find SELECT keyword before the given position
    fn find_select_before(&self, pos: usize) -> Option<usize> {
        self.chunks[..pos]
            .iter()
            .enumerate()
            .rev()
            .find_map(|(i, chunk)| match chunk {
                SQLChunk::Text(text) if text.trim().eq_ignore_ascii_case("SELECT") => Some(i),
                _ => None,
            })
    }

    /// Find table after FROM keyword starting from SELECT position
    fn find_table_after_from(&self, select_pos: usize) -> Option<&'a dyn SQLTableInfo> {
        let chunks = &self.chunks[select_pos..];
        let mut found_from = false;

        for chunk in chunks {
            match chunk {
                SQLChunk::Text(text) if text.trim().eq_ignore_ascii_case("FROM") => {
                    found_from = true;
                }
                SQLChunk::Table(table) if found_from => return Some(*table),
                _ => {}
            }
        }
        None
    }

    /// Smart capacity estimation
    fn estimate_capacity(&self) -> usize {
        self.chunks
            .iter()
            .map(|chunk| match chunk {
                SQLChunk::Text(t) => t.len(),
                SQLChunk::Param { .. } => 1,
                SQLChunk::SQL(sql) => sql.chunks.len() << 3, // * 8
                SQLChunk::Table(t) => t.name().len() + 2,
                SQLChunk::Column(c) => c.table().name().len() + c.name().len() + 5,
                SQLChunk::Alias { alias, .. } => alias.len() + 4,
                SQLChunk::Subquery(sql) => (sql.chunks.len() << 3) + 2,
            })
            .sum::<usize>()
            + self.chunks.len()
    }

    /// Check if space needed between chunks
    fn needs_space(&self, chunk: &SQLChunk<'a, V>, index: usize) -> bool {
        let current_no_space = matches!(chunk, SQLChunk::Text(t) if t.ends_with(['(', ',', ' ']));
        let next_no_space = matches!(
            self.chunks.get(index + 1),
            Some(SQLChunk::Text(t)) if t.starts_with([')', ',', ' '])
        );
        !current_no_space && !next_no_space
    }

    /// Returns references to parameter values from this SQL fragment in the correct order.
    pub fn params<'b>(&'b self) -> Vec<&'b V> {
        let mut params_vec = Vec::with_capacity(self.chunks.len().min(8));
        for chunk in &self.chunks {
            params_vec.extend(Self::collect_chunk_params(chunk));
        }
        params_vec
    }

    /// Creates an aliased version of this SQL using the Alias chunk
    pub fn alias(self, alias: impl Into<CompactString>) -> SQL<'a, V> {
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

    /// Creates a comma-separated list of parameter placeholders with values: "?, ?, ?"
    pub fn parameters<I>(values: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Cow<'a, V>>,
    {
        let mut values_iter = values.into_iter();

        match values_iter.next() {
            None => Self::empty(),
            Some(first_value) => match values_iter.next() {
                None => Self::parameter(first_value),
                Some(second_value) => {
                    let mut chunks = SmallVec::new();
                    chunks.push(SQLChunk::Param(Param {
                        value: Some(first_value.into()),
                        placeholder: Self::POSITIONAL_PLACEHOLDER,
                    }));
                    chunks.push(SQLChunk::Text(Cow::Owned(CompactString::const_new(", "))));
                    chunks.push(SQLChunk::Param(Param {
                        value: Some(second_value.into()),
                        placeholder: Self::POSITIONAL_PLACEHOLDER,
                    }));

                    for value in values_iter {
                        chunks.push(SQLChunk::Text(Cow::Owned(CompactString::const_new(", "))));
                        chunks.push(SQLChunk::Param(Param {
                            value: Some(value.into()),
                            placeholder: Self::POSITIONAL_PLACEHOLDER,
                        }));
                    }
                    SQL { chunks }
                }
            },
        }
    }

    /// Creates a comma-separated list of column assignments: "col1 = ?, col2 = ?"
    pub fn assignments<I, T>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (&'a str, T)>,
        T: Into<Cow<'a, V>>,
    {
        let mut pairs_iter = pairs.into_iter();

        match pairs_iter.next() {
            None => Self::empty(),
            Some((first_col, first_val)) => match pairs_iter.next() {
                None => Self::raw(first_col)
                    .append_raw(" = ")
                    .append(Self::parameter(first_val.into())),
                Some((second_col, second_val)) => {
                    let mut chunks = SmallVec::new();
                    chunks.push(SQLChunk::Text(Cow::Owned(first_col.to_compact_string())));
                    chunks.push(SQLChunk::Text(Cow::Owned(CompactString::const_new(" = "))));
                    chunks.push(SQLChunk::Param(Param {
                        value: Some(first_val.into()),
                        placeholder: Self::POSITIONAL_PLACEHOLDER,
                    }));
                    chunks.push(SQLChunk::Text(Cow::Owned(CompactString::const_new(", "))));
                    chunks.push(SQLChunk::Text(Cow::Owned(second_col.to_compact_string())));
                    chunks.push(SQLChunk::Text(Cow::Owned(CompactString::const_new(" = "))));
                    chunks.push(SQLChunk::Param(Param {
                        value: Some(second_val.into()),
                        placeholder: Self::POSITIONAL_PLACEHOLDER,
                    }));

                    for (col, val) in pairs_iter {
                        chunks.push(SQLChunk::Text(Cow::Owned(CompactString::const_new(", "))));
                        chunks.push(SQLChunk::Text(Cow::Owned(col.to_compact_string())));
                        chunks.push(SQLChunk::Text(Cow::Owned(CompactString::const_new(" = "))));
                        chunks.push(SQLChunk::Param(Param {
                            value: Some(val.into()),
                            placeholder: Self::POSITIONAL_PLACEHOLDER,
                        }));
                    }
                    SQL { chunks }
                }
            },
        }
    }
}

impl<'a, V: SQLParam> IntoIterator for SQL<'a, V> {
    type Item = SQLChunk<'a, V>;
    type IntoIter = std::iter::Flatten<
        std::iter::Map<
            smallvec::IntoIter<[SQLChunk<'a, V>; 3]>,
            fn(SQLChunk<'a, V>) -> Box<dyn Iterator<Item = SQLChunk<'a, V>> + 'a>,
        >,
    >;

    fn into_iter(self) -> Self::IntoIter {
        fn flatten_chunk<'a, V: SQLParam>(
            chunk: SQLChunk<'a, V>,
        ) -> Box<dyn Iterator<Item = SQLChunk<'a, V>> + 'a> {
            match chunk {
                SQLChunk::SQL(nested_sql) => Box::new(nested_sql.into_iter()),
                other => Box::new(std::iter::once(other)),
            }
        }

        self.chunks
            .into_iter()
            .map(flatten_chunk as fn(_) -> _)
            .flatten()
    }
}

impl<'a, V: SQLParam> Default for SQL<'a, V> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<'a, V: SQLParam + 'a> From<&'a str> for SQL<'a, V> {
    fn from(s: &'a str) -> Self {
        SQL::raw(s)
    }
}

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
        SQL::empty()
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
        let params = self.params();
        write!(f, r#"sql: "{}", params: {:?} "#, self.sql(), params)
    }
}

impl<'a, V: SQLParam + std::fmt::Debug> std::fmt::Debug for SQL<'a, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SQL")
            .field("sql", &self.sql())
            .field("chunk_count", &self.chunks.len())
            .finish()
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

impl<'a, V, T, const N: usize> ToSQL<'a, V> for &[T; N]
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
        SQL::table(*self)
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for &'a dyn SQLColumnInfo {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::column(*self)
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for Box<[&'a dyn SQLColumnInfo]> {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::join(self.iter().map(|&v| SQL::column(v)), ", ")
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for Box<[&'a dyn SQLTableInfo]> {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::join(self.iter().map(|&v| SQL::table(v)), ", ")
    }
}

// Implement ToSQL for primitive types
impl<'a, V> ToSQL<'a, V> for &'a str
where
    V: SQLParam + 'a,
    V: From<&'a str>,
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(V::from(self))
    }
}

impl<'a, V> ToSQL<'a, V> for String
where
    V: SQLParam + 'a,
    V: From<String>,
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(V::from(self.clone()))
    }
}

impl<'a, V> ToSQL<'a, V> for i32
where
    V: SQLParam + 'a + From<i64>,
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(V::from(*self as i64))
    }
}

impl<'a, V> ToSQL<'a, V> for i64
where
    V: SQLParam + 'a + From<i64>,
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(V::from(*self))
    }
}

impl<'a, V> ToSQL<'a, V> for f64
where
    V: SQLParam + 'a + From<f64>,
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(V::from(*self))
    }
}

impl<'a, V> ToSQL<'a, V> for bool
where
    V: SQLParam + 'a + From<i64>,
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(V::from(*self as i64))
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
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(V::from(*self))
    }
}

#[cfg(feature = "uuid")]
impl<'a, V> ToSQL<'a, V> for &'a Uuid
where
    V: SQLParam + 'a,
    V: From<&'a Uuid>,
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(V::from(*self))
    }
}

pub mod placeholders {
    use super::{Placeholder, PlaceholderStyle};

    pub const fn colon(name: &'static str) -> Placeholder {
        Placeholder::with_style(name, PlaceholderStyle::Colon)
    }

    pub const fn at(name: &'static str) -> Placeholder {
        Placeholder::with_style(name, PlaceholderStyle::AtSign)
    }

    pub const fn dollar(name: &'static str) -> Placeholder {
        Placeholder::with_style(name, PlaceholderStyle::Dollar)
    }
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
                alias: CompactString::const_new(self.name),
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
