use crate::prelude::*;
use crate::{Param, Placeholder, SQLColumnInfo, SQLParam, SQLTableInfo, sql::tokens::Token};

/// A SQL chunk represents a part of an SQL statement.
///
/// This enum has 7 variants, each with a clear semantic purpose:
/// - `Token` - SQL keywords and operators (SELECT, FROM, =, etc.)
/// - `Ident` - Quoted identifiers ("table_name", "column_name")
/// - `Raw` - Unquoted raw SQL text (function names, expressions)
/// - `Param` - Parameter placeholders with values
/// - `Table` - Table reference with metadata access
/// - `Column` - Column reference with metadata access
/// - `Alias` - Alias wrapper (expr AS "name")
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

    /// Parameter with value and placeholder
    /// Renders as: ? or $1 or :name depending on placeholder style
    Param(Param<'a, V>),

    /// Table reference with full metadata access
    /// Renders as: "table_name"
    /// Provides: columns() for SELECT *, dependencies() for FK tracking
    Table(&'static dyn SQLTableInfo),

    /// Column reference with full metadata access
    /// Renders as: "table"."column"
    /// Provides: table(), is_primary_key(), foreign_key(), etc.
    Column(&'static dyn SQLColumnInfo),

    /// Alias wrapper: renders inner chunk followed by AS "alias"
    /// Renders as: {inner} AS "alias"
    Alias {
        inner: Box<SQLChunk<'a, V>>,
        alias: Cow<'a, str>,
    },
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
    pub const fn table(table: &'static dyn SQLTableInfo) -> Self {
        Self::Table(table)
    }

    /// Creates a column chunk - const
    #[inline]
    pub const fn column(column: &'static dyn SQLColumnInfo) -> Self {
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

    /// Creates a parameter chunk with owned value
    #[inline]
    pub fn param(value: impl Into<Cow<'a, V>>, placeholder: Placeholder) -> Self {
        Self::Param(Param {
            value: Some(value.into()),
            placeholder,
        })
    }

    /// Creates an alias chunk wrapping any SQLChunk
    #[inline]
    pub fn alias(inner: SQLChunk<'a, V>, alias: impl Into<Cow<'a, str>>) -> Self {
        Self::Alias {
            inner: Box::new(inner),
            alias: alias.into(),
        }
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
            SQLChunk::Param(Param { placeholder, .. }) => {
                let _ = write!(buf, "{}", placeholder);
            }
            SQLChunk::Table(table) => {
                let _ = buf.write_char('"');
                let _ = buf.write_str(table.name());
                let _ = buf.write_char('"');
            }
            SQLChunk::Column(column) => {
                let _ = buf.write_char('"');
                let _ = buf.write_str(column.table().name());
                let _ = buf.write_str("\".\"");
                let _ = buf.write_str(column.name());
                let _ = buf.write_char('"');
            }
            SQLChunk::Alias { inner, alias } => {
                inner.write(buf);
                let _ = buf.write_str(" AS \"");
                let _ = buf.write_str(alias);
                let _ = buf.write_char('"');
            }
        }
    }

    /// Check if this chunk is "word-like" (needs space separation from other word-like chunks)
    #[inline]
    pub(crate) const fn is_word_like(&self) -> bool {
        match self {
            SQLChunk::Token(t) => !matches!(
                t,
                Token::LPAREN
                    | Token::RPAREN
                    | Token::COMMA
                    | Token::SEMI
                    | Token::DOT
                    | Token::EQ
                    | Token::NE
                    | Token::LT
                    | Token::GT
                    | Token::LE
                    | Token::GE
            ),
            SQLChunk::Ident(_)
            | SQLChunk::Raw(_)
            | SQLChunk::Param(_)
            | SQLChunk::Table(_)
            | SQLChunk::Column(_)
            | SQLChunk::Alias { .. } => true,
        }
    }
}

impl<'a, V: SQLParam + core::fmt::Debug> core::fmt::Debug for SQLChunk<'a, V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SQLChunk::Token(token) => f.debug_tuple("Token").field(token).finish(),
            SQLChunk::Ident(name) => f.debug_tuple("Ident").field(name).finish(),
            SQLChunk::Raw(text) => f.debug_tuple("Raw").field(text).finish(),
            SQLChunk::Param(param) => f.debug_tuple("Param").field(param).finish(),
            SQLChunk::Table(table) => f.debug_tuple("Table").field(&table.name()).finish(),
            SQLChunk::Column(column) => f
                .debug_tuple("Column")
                .field(&format!("{}.{}", column.table().name(), column.name()))
                .finish(),
            SQLChunk::Alias { inner, alias } => f
                .debug_struct("Alias")
                .field("inner", inner)
                .field("alias", alias)
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

impl<'a, V: SQLParam> From<&'static dyn SQLColumnInfo> for SQLChunk<'a, V> {
    #[inline]
    fn from(value: &'static dyn SQLColumnInfo) -> Self {
        Self::Column(value)
    }
}

impl<'a, V: SQLParam> From<&'static dyn SQLTableInfo> for SQLChunk<'a, V> {
    #[inline]
    fn from(value: &'static dyn SQLTableInfo) -> Self {
        Self::Table(value)
    }
}

impl<'a, V: SQLParam> From<Param<'a, V>> for SQLChunk<'a, V> {
    #[inline]
    fn from(value: Param<'a, V>) -> Self {
        Self::Param(value)
    }
}
