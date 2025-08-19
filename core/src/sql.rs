use compact_str::{CompactString, ToCompactString};
use smallvec::{SmallVec, smallvec};
use std::{borrow::Cow, collections::HashMap, fmt::Display};

use crate::{
    OwnedParam, Param, ParamBind, Placeholder, PlaceholderStyle,
    traits::{SQLColumnInfo, SQLParam, SQLTableInfo},
};

/// A SQL chunk represents a part of an SQL statement.
#[derive(Clone)]
pub enum SQLChunk<'a, V: SQLParam + 'a> {
    Text(Cow<'a, CompactString>),
    Param(Param<'a, V>),
    SQL(Box<SQL<'a, V>>),
    /// A table reference that can render itself with proper schema/alias handling
    Table(&'static dyn SQLTableInfo),
    /// A column reference that can render itself with proper table qualification
    Column(&'static dyn SQLColumnInfo),
    /// An alias wrapping any SQL chunk: "chunk AS alias"
    Alias {
        chunk: Box<SQLChunk<'a, V>>,
        alias: CompactString,
    },
    /// A subquery wrapped in parentheses: "(SELECT ...)"
    Subquery(Box<SQL<'a, V>>),
}

pub enum OwnedSQLChunk<V: SQLParam> {
    Text(CompactString),
    Param(OwnedParam<V>),
    SQL(Box<OwnedSQL<V>>),
    Table(&'static dyn SQLTableInfo),
    Column(&'static dyn SQLColumnInfo),
    Alias {
        chunk: Box<OwnedSQLChunk<V>>,
        alias: CompactString,
    },
    Subquery(Box<OwnedSQL<V>>),
}

impl<'a, V: SQLParam> From<SQLChunk<'a, V>> for OwnedSQLChunk<V> {
    fn from(value: SQLChunk<'a, V>) -> Self {
        match value {
            SQLChunk::Text(cow) => Self::Text(cow.into_owned()),
            SQLChunk::Param(param) => Self::Param(param.into()),
            SQLChunk::SQL(sql) => Self::SQL(Box::new((*sql).into())),
            SQLChunk::Table(sqltable_info) => Self::Table(sqltable_info),
            SQLChunk::Column(sqlcolumn_info) => Self::Column(sqlcolumn_info),
            SQLChunk::Alias { chunk, alias } => Self::Alias {
                chunk: Box::new((*chunk).into()),
                alias,
            },
            SQLChunk::Subquery(sql) => Self::Subquery(Box::new((*sql).into())),
        }
    }
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
            placeholder,
        })
    }

    /// Creates a nested SQL chunk
    pub fn sql(sql: SQL<'a, V>) -> Self {
        Self::SQL(Box::new(sql))
    }

    /// Creates a table chunk
    pub const fn table(table: &'static dyn SQLTableInfo) -> Self {
        Self::Table(table)
    }

    /// Creates a column chunk
    pub const fn column(column: &'static dyn SQLColumnInfo) -> Self {
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
    pub(crate) fn write_to_buffer(&self, buf: &mut CompactString) {
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
#[derive(Debug, Clone)]
pub struct SQL<'a, V: SQLParam> {
    /// The chunks that make up this SQL statement or fragment.
    pub chunks: SmallVec<[SQLChunk<'a, V>; 3]>,
}

pub struct OwnedSQL<V: SQLParam> {
    pub chunks: SmallVec<[OwnedSQLChunk<V>; 3]>,
}

impl<'a, V: SQLParam> From<SQL<'a, V>> for OwnedSQL<V> {
    fn from(value: SQL<'a, V>) -> Self {
        Self {
            chunks: value.chunks.iter().map(|v| v.clone().into()).collect(),
        }
    }
}

impl<'a, V: SQLParam> SQL<'a, V> {
    /// Const placeholder instances for zero-copy usage
    const POSITIONAL_PLACEHOLDER: Placeholder = Placeholder::positional();

    pub const fn new<'b>(chunks: [SQLChunk<'b, V>; 3]) -> SQL<'b, V> {
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

    pub fn into_owned(&self) -> OwnedSQL<V> {
        OwnedSQL::from(self.clone())
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
        let sql = Cow::Owned(sql.as_ref().to_compact_string());

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
    pub fn table(table: &'static dyn SQLTableInfo) -> SQL<'a, V> {
        SQL {
            chunks: smallvec![SQLChunk::table(table)],
        }
    }

    pub fn column(column: &'static dyn SQLColumnInfo) -> SQL<'a, V> {
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
    pub fn append_raw(mut self, sql: impl AsRef<str>) -> Self {
        let text_chunk = SQLChunk::Text(Cow::Owned(sql.as_ref().to_compact_string()));
        self.chunks.push(text_chunk);
        self
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
        T::Item: crate::ToSQL<'a, V>,
    {
        let sqls: Vec<_> = sqls.into_iter().map(|sql| sql.to_sql()).collect();

        if sqls.is_empty() {
            return SQL::empty();
        }

        if sqls.len() == 1 {
            return sqls.into_iter().next().unwrap();
        }

        // Pre-calculate capacity: sum of all chunks + separators
        let total_chunks =
            sqls.iter().map(|sql| sql.chunks.len()).sum::<usize>() + (sqls.len() - 1);
        let mut chunks = SmallVec::with_capacity(total_chunks);

        let separator_chunk = SQLChunk::Text(Cow::Owned(CompactString::const_new(separator)));

        for (i, sql) in sqls.into_iter().enumerate() {
            if i > 0 {
                chunks.push(separator_chunk.clone());
            }
            chunks.extend(sql.chunks);
        }

        SQL { chunks }
    }

    /// Collects parameter references from a single chunk
    fn collect_chunk_params<'b>(chunk: &'b SQLChunk<'a, V>, params_vec: &mut Vec<&'b V>) {
        match chunk {
            SQLChunk::Param(Param {
                value: Some(value), ..
            }) => params_vec.push(value.as_ref()),
            SQLChunk::SQL(sql) => params_vec.extend(sql.params()),
            SQLChunk::Alias { chunk, .. } => Self::collect_chunk_params(chunk, params_vec),
            SQLChunk::Subquery(sql) => params_vec.extend(sql.params()),
            _ => {}
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
                    if let Some(name) = param.placeholder.name
                        && let Some(value) = param_map.get(name)
                    {
                        param.value = Some(Cow::Owned(value.clone()));
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
    pub(crate) fn write_qualified_columns(
        &self,
        buf: &mut CompactString,
        table: &'a dyn SQLTableInfo,
    ) {
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

    fn write_sql(&self, buf: &mut CompactString) {
        for i in 0..self.chunks.len() {
            self.write_chunk(buf, &self.chunks[i], i);

            if self.needs_space(i) {
                buf.push(' ');
            }
        }
    }

    /// Write a single chunk to buffer with pattern detection
    pub(crate) fn write_chunk(
        &self,
        buf: &mut CompactString,
        chunk: &SQLChunk<'a, V>,
        index: usize,
    ) {
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
                } else if self.detect_select_from_non_table_pattern(index) {
                    buf.push_str("SELECT *");
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
    pub(crate) fn detect_pattern_at(&self, empty_index: usize) -> Option<&'a dyn SQLTableInfo> {
        let select_pos = self.find_select_before(empty_index)?;
        self.find_table_after_from(select_pos)
    }

    /// Detect SELECT-FROM-TABLE pattern starting from SELECT index
    pub(crate) fn detect_select_from_table_pattern(
        &self,
        select_index: usize,
    ) -> Option<&'a dyn SQLTableInfo> {
        if select_index + 2 < self.chunks.len()
            && let (SQLChunk::Text(from_text), SQLChunk::Table(table)) = (
                &self.chunks[select_index + 1],
                &self.chunks[select_index + 2],
            )
            && from_text.trim().eq_ignore_ascii_case("FROM")
        {
            return Some(*table);
        }
        None
    }

    /// Detect SELECT-FROM-NON_TABLE pattern (e.g., CTE, subquery, etc.)
    fn detect_select_from_non_table_pattern(&self, select_index: usize) -> bool {
        if select_index + 2 < self.chunks.len() {
            if let SQLChunk::Text(from_text) = &self.chunks[select_index + 1] {
                if from_text.trim().eq_ignore_ascii_case("FROM") {
                    // Check if what follows FROM is NOT a table
                    match &self.chunks[select_index + 2] {
                        SQLChunk::Table(_) => false, // This is a table, handled by other pattern
                        SQLChunk::Text(_) | SQLChunk::SQL(_) | SQLChunk::Subquery(_) => true, // CTE name, subquery, etc.
                        _ => true, // Any other chunk type is not a table
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
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
        let chunk_content_size: usize = self
            .chunks
            .iter()
            .map(|chunk| match chunk {
                SQLChunk::Text(t) => t.len(),
                SQLChunk::Param { .. } => 1, // Single placeholder character
                SQLChunk::SQL(sql) => sql.chunks.len() * 8, // Average 8 chars per nested chunk
                SQLChunk::Table(t) => t.name().len() + 2, // Quotes around table name
                SQLChunk::Column(c) => c.table().name().len() + c.name().len() + 5, // "table"."column"
                SQLChunk::Alias { alias, .. } => alias.len() + 4, // " AS " + alias
                SQLChunk::Subquery(sql) => (sql.chunks.len() * 8) + 2, // Parentheses + content
            })
            .sum();

        // Add space for potential spaces between chunks
        chunk_content_size + self.chunks.len()
    }

    pub(crate) fn needs_space(&self, index: usize) -> bool {
        if index + 1 >= self.chunks.len() {
            return false;
        }

        let current = &self.chunks[index];

        // Find next non-empty chunk
        let mut next_index = index + 1;
        let next = loop {
            if next_index >= self.chunks.len() {
                return false;
            }

            let candidate = &self.chunks[next_index];
            if let SQLChunk::Text(t) = candidate
                && t.is_empty()
            {
                next_index += 1;
                continue;
            }
            break candidate;
        };

        let ends_word = chunk_ends_word(current);
        let starts_word = chunk_starts_word(next);

        ends_word && starts_word
    }

    /// Returns references to parameter values from this SQL fragment in the correct order.
    pub fn params(&self) -> Vec<&V> {
        let mut params_vec = Vec::with_capacity(self.chunks.len().min(8));
        for chunk in &self.chunks {
            Self::collect_chunk_params(chunk, &mut params_vec);
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
        let values: Vec<_> = values.into_iter().map(|v| v.into()).collect();

        if values.is_empty() {
            return Self::empty();
        }

        if values.len() == 1 {
            return Self::parameter(values.into_iter().next().unwrap());
        }

        // Pre-calculate capacity: each value creates a param chunk, plus separators
        let mut chunks = SmallVec::with_capacity(values.len() * 2 - 1);
        let separator_chunk = SQLChunk::Text(Cow::Owned(CompactString::const_new(", ")));

        for (i, value) in values.into_iter().enumerate() {
            if i > 0 {
                chunks.push(separator_chunk.clone());
            }
            chunks.push(SQLChunk::Param(Param {
                value: Some(value),
                placeholder: Self::POSITIONAL_PLACEHOLDER,
            }));
        }

        SQL { chunks }
    }

    /// Creates a comma-separated list of column assignments: "col1 = ?, col2 = ?"
    pub fn assignments<I, T>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (&'a str, T)>,
        T: Into<Cow<'a, V>>,
    {
        let pairs: Vec<_> = pairs
            .into_iter()
            .map(|(col, val)| (col, val.into()))
            .collect();

        if pairs.is_empty() {
            return Self::empty();
        }

        if pairs.len() == 1 {
            let (col, val) = pairs.into_iter().next().unwrap();
            return Self::raw(col)
                .append_raw(" = ")
                .append(Self::parameter(val));
        }

        // Pre-calculate capacity: each pair creates 3 chunks (col, " = ", param), plus separators
        let mut chunks = SmallVec::with_capacity(pairs.len() * 4 - 1);
        let separator_chunk = SQLChunk::Text(Cow::Owned(CompactString::const_new(", ")));
        let equals_chunk = SQLChunk::Text(Cow::Owned(CompactString::const_new(" = ")));

        for (i, (col, val)) in pairs.into_iter().enumerate() {
            if i > 0 {
                chunks.push(separator_chunk.clone());
            }
            chunks.push(SQLChunk::Text(Cow::Owned(col.to_compact_string())));
            chunks.push(equals_chunk.clone());
            chunks.push(SQLChunk::Param(Param {
                value: Some(val),
                placeholder: Self::POSITIONAL_PLACEHOLDER,
            }));
        }

        SQL { chunks }
    }
}

/// Helper function to determine if a chunk ends with a word character
fn chunk_ends_word<V: SQLParam>(chunk: &SQLChunk<'_, V>) -> bool {
    match chunk {
        SQLChunk::Text(t) => {
            let last = t.chars().last().unwrap_or(' ');
            !last.is_whitespace() && !['(', ',', '.', ')'].contains(&last)
        }
        SQLChunk::Table(_)
        | SQLChunk::Column(_)
        | SQLChunk::Param(_)
        | SQLChunk::Alias { .. }
        | SQLChunk::Subquery(_) => true,
        _ => false,
    }
}

/// Helper function to determine if a chunk starts with a word character  
fn chunk_starts_word<V: SQLParam>(chunk: &SQLChunk<'_, V>) -> bool {
    match chunk {
        SQLChunk::Text(t) => {
            let first = t.chars().next().unwrap_or(' ');
            !first.is_whitespace() && !['(', ',', ')', ';'].contains(&first)
        }
        SQLChunk::Table(_)
        | SQLChunk::Column(_)
        | SQLChunk::Param(_)
        | SQLChunk::Alias { .. }
        | SQLChunk::Subquery(_) => true,
        _ => false,
    }
}

impl<'a, V: SQLParam> IntoIterator for SQL<'a, V> {
    type Item = SQLChunk<'a, V>;
    type IntoIter = std::iter::FlatMap<
        smallvec::IntoIter<[SQLChunk<'a, V>; 3]>,
        Box<dyn Iterator<Item = SQLChunk<'a, V>> + 'a>,
        fn(SQLChunk<'a, V>) -> Box<dyn Iterator<Item = SQLChunk<'a, V>> + 'a>,
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
            .flat_map(flatten_chunk as fn(_) -> _)
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

use crate::ToSQL;

impl<V: SQLParam> ToSQL<'static, V> for OwnedSQL<V> {
    fn to_sql(&self) -> SQL<'static, V> {
        SQL::from(self)
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for SQL<'a, V> {
    fn to_sql(&self) -> SQL<'a, V> {
        self.clone()
    }
}
