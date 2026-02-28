use crate::prelude::*;
use crate::{Param, Placeholder, SQLParam, sql::tokens::Token};

/// Lightweight table reference for SQL rendering.
///
/// Contains the table name and column names needed for SQL generation
/// (e.g. SELECT * expansion) without dynamic dispatch.
#[derive(Clone, Copy, Debug)]
pub struct TableRef {
    pub name: &'static str,
    pub column_names: &'static [&'static str],
}

/// Lightweight column reference for SQL rendering.
///
/// Contains the table and column names needed for qualified column
/// references (e.g. `"table"."column"`) without dynamic dispatch.
#[derive(Clone, Copy, Debug)]
pub struct ColumnRef {
    pub table_name: &'static str,
    pub column_name: &'static str,
}

/// A SQL chunk represents a part of an SQL statement.
///
/// Each variant has a clear semantic purpose:
/// - `Token` - SQL keywords and operators (SELECT, FROM, =, etc.)
/// - `Ident` - Quoted identifiers ("table_name", "column_name")
/// - `Raw` - Unquoted raw SQL text (function names, expressions)
/// - `Param` - Parameter placeholders with values
/// - `Table` - Table reference via lightweight `TableRef`
/// - `Column` - Column reference via lightweight `ColumnRef`
#[derive(Clone)]
pub enum SQLChunk<'a, V: SQLParam> {
    /// SQL keywords and operators: SELECT, FROM, WHERE, =, AND, etc.
    /// Renders as: keyword with automatic spacing rules
    Token(Token),

    /// Quoted identifier for user-provided names
    /// Renders as: "name" (with quotes)
    /// Use for: table names, column names, alias names
    Ident(Cow<'a, str>),

    /// Raw SQL text (unquoted) for expressions, function names
    /// Renders as: text (no quotes, as-is)
    /// Use for: function names like COUNT, expressions, numeric literals
    Raw(Cow<'a, str>),

    /// Unsigned integer SQL literal rendered directly without heap allocation.
    ///
    /// Primarily used for clauses like LIMIT/OFFSET where numeric literals are
    /// embedded directly in SQL text rather than parameterized.
    Number(usize),

    /// Parameter with value and placeholder
    /// Renders as: ? or $1 or :name depending on placeholder style
    Param(Param<'a, V>),

    /// Table reference with static name and column names.
    /// Renders as: "table_name"
    /// Column names used for SELECT * expansion.
    Table(TableRef),

    /// Column reference with static table and column names.
    /// Renders as: "table_name"."column_name"
    Column(ColumnRef),
}

impl<'a, V: SQLParam> SQLChunk<'a, V> {
    // ==================== const constructors ====================

    /// Creates a token chunk - const
    #[inline]
    pub const fn token(t: Token) -> Self {
        Self::Token(t)
    }

    /// Creates a quoted identifier from a static string - const
    #[inline]
    pub const fn ident_static(name: &'static str) -> Self {
        Self::Ident(Cow::Borrowed(name))
    }

    /// Creates raw SQL text from a static string - const
    #[inline]
    pub const fn raw_static(text: &'static str) -> Self {
        Self::Raw(Cow::Borrowed(text))
    }

    /// Creates a table chunk - const
    #[inline]
    pub const fn table(table: TableRef) -> Self {
        Self::Table(table)
    }

    /// Creates a column chunk - const
    #[inline]
    pub const fn column(column: ColumnRef) -> Self {
        Self::Column(column)
    }

    /// Creates a parameter chunk with borrowed value - const
    #[inline]
    pub const fn param_borrowed(value: &'a V, placeholder: Placeholder) -> Self {
        Self::Param(Param {
            value: Some(Cow::Borrowed(value)),
            placeholder,
        })
    }

    // ==================== non-const constructors ====================

    /// Creates a quoted identifier from a runtime string
    #[inline]
    pub fn ident(name: impl Into<Cow<'a, str>>) -> Self {
        Self::Ident(name.into())
    }

    /// Creates raw SQL text from a runtime string
    #[inline]
    pub fn raw(text: impl Into<Cow<'a, str>>) -> Self {
        Self::Raw(text.into())
    }

    /// Creates an unsigned integer SQL literal chunk.
    #[inline]
    pub const fn number(value: usize) -> Self {
        Self::Number(value)
    }

    /// Creates a parameter chunk with owned value
    #[inline]
    pub fn param(value: impl Into<Cow<'a, V>>, placeholder: Placeholder) -> Self {
        Self::Param(Param {
            value: Some(value.into()),
            placeholder,
        })
    }

    // ==================== write implementation ====================

    /// Write chunk content to buffer
    pub(crate) fn write(&self, buf: &mut impl core::fmt::Write) {
        match self {
            SQLChunk::Token(token) => {
                let _ = buf.write_str(token.as_str());
            }
            SQLChunk::Ident(name) => {
                let _ = buf.write_char('"');
                let _ = buf.write_str(name);
                let _ = buf.write_char('"');
            }
            SQLChunk::Raw(text) => {
                let _ = buf.write_str(text);
            }
            SQLChunk::Number(value) => {
                let _ = write!(buf, "{}", value);
            }
            SQLChunk::Param(Param { placeholder, .. }) => {
                let _ = write!(buf, "{}", placeholder);
            }
            SQLChunk::Table(t) => {
                let _ = buf.write_char('"');
                let _ = buf.write_str(t.name);
                let _ = buf.write_char('"');
            }
            SQLChunk::Column(c) => {
                let _ = buf.write_char('"');
                let _ = buf.write_str(c.table_name);
                let _ = buf.write_str("\".\"");
                let _ = buf.write_str(c.column_name);
                let _ = buf.write_char('"');
            }
        }
    }

    /// Check if this chunk is "word-like" (needs space separation from other word-like chunks)
    #[inline]
    pub(crate) const fn is_word_like(&self) -> bool {
        match self {
            SQLChunk::Token(t) => !t.is_punctuation() && !t.is_operator(),
            SQLChunk::Ident(_)
            | SQLChunk::Raw(_)
            | SQLChunk::Number(_)
            | SQLChunk::Param(_)
            | SQLChunk::Table(_)
            | SQLChunk::Column(_) => true,
        }
    }
}

impl<'a, V: SQLParam + core::fmt::Debug> core::fmt::Debug for SQLChunk<'a, V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SQLChunk::Token(token) => f.debug_tuple("Token").field(token).finish(),
            SQLChunk::Ident(name) => f.debug_tuple("Ident").field(name).finish(),
            SQLChunk::Raw(text) => f.debug_tuple("Raw").field(text).finish(),
            SQLChunk::Number(value) => f.debug_tuple("Number").field(value).finish(),
            SQLChunk::Param(param) => f.debug_tuple("Param").field(param).finish(),
            SQLChunk::Table(t) => f.debug_tuple("Table").field(&t.name).finish(),
            SQLChunk::Column(c) => f
                .debug_tuple("Column")
                .field(&format!("{}.{}", c.table_name, c.column_name))
                .finish(),
        }
    }
}

// ==================== From implementations ====================

impl<'a, V: SQLParam> From<Token> for SQLChunk<'a, V> {
    #[inline]
    fn from(value: Token) -> Self {
        Self::Token(value)
    }
}

impl<'a, V: SQLParam> From<TableRef> for SQLChunk<'a, V> {
    #[inline]
    fn from(value: TableRef) -> Self {
        Self::Table(value)
    }
}

impl<'a, V: SQLParam> From<ColumnRef> for SQLChunk<'a, V> {
    #[inline]
    fn from(value: ColumnRef) -> Self {
        Self::Column(value)
    }
}

impl<'a, V: SQLParam> From<Param<'a, V>> for SQLChunk<'a, V> {
    #[inline]
    fn from(value: Param<'a, V>) -> Self {
        Self::Param(value)
    }
}
