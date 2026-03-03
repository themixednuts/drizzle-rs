use crate::SQLConstraintKind;
use crate::prelude::*;
use crate::{Param, Placeholder, SQLParam, sql::tokens::Token};

// ==================== Dialect enums ====================

/// Dialect-specific column metadata. Const-compatible enum grouping
/// fields that only apply to one dialect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnDialect {
    SQLite {
        autoincrement: bool,
    },
    PostgreSQL {
        postgres_type: &'static str,
        is_serial: bool,
        is_bigserial: bool,
        is_generated_identity: bool,
        is_identity_always: bool,
    },
}

/// Dialect-specific table metadata.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TableDialect {
    #[default]
    PostgreSQL,
    SQLite {
        without_rowid: bool,
        strict: bool,
    },
}

// ==================== Ref structs ====================

/// Foreign key reference as a const Copy struct.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForeignKeyRef {
    pub target_table: &'static str,
    pub source_columns: &'static [&'static str],
    pub target_columns: &'static [&'static str],
}

/// Primary key reference as a const Copy struct.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrimaryKeyRef {
    pub columns: &'static [&'static str],
}

/// Constraint reference as a const Copy struct.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstraintRef {
    pub name: Option<&'static str>,
    pub kind: SQLConstraintKind,
    pub columns: &'static [&'static str],
    pub check_expression: Option<&'static str>,
}

// ==================== Enhanced TableRef and ColumnRef ====================

/// Table reference with full schema metadata.
///
/// Carries both the SQL rendering fields (`name`, `column_names`) and
/// complete schema metadata (columns, keys, constraints). SQL rendering
/// code only uses `name`/`column_names` and ignores extra fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableRef {
    // SQL rendering fields
    pub name: &'static str,
    pub column_names: &'static [&'static str],

    // Schema metadata
    pub schema: Option<&'static str>,
    pub qualified_name: &'static str,
    pub columns: &'static [ColumnRef],
    pub primary_key: Option<PrimaryKeyRef>,
    pub foreign_keys: &'static [ForeignKeyRef],
    pub constraints: &'static [ConstraintRef],
    pub dependency_names: &'static [&'static str],

    // Dialect-specific
    pub dialect: TableDialect,
}

impl TableRef {
    /// Creates a lightweight `TableRef` for SQL rendering only.
    ///
    /// Only `name` and `column_names` are populated; metadata fields use
    /// empty defaults. Use a full struct literal for metadata-carrying refs.
    pub const fn sql(name: &'static str, column_names: &'static [&'static str]) -> Self {
        Self {
            name,
            column_names,
            schema: None,
            qualified_name: "",
            columns: &[],
            primary_key: None,
            foreign_keys: &[],
            constraints: &[],
            dependency_names: &[],
            dialect: TableDialect::PostgreSQL,
        }
    }
}

/// Column reference with full schema metadata.
///
/// Carries both the SQL rendering fields (`table`, `name`) and
/// complete column metadata. SQL rendering code only uses the name fields
/// and ignores extra metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnRef {
    // SQL rendering fields
    pub table: &'static str,
    pub name: &'static str,

    // Schema metadata
    pub sql_type: &'static str,
    pub not_null: bool,
    pub primary_key: bool,
    pub unique: bool,
    pub has_default: bool,

    // Dialect-specific
    pub dialect: ColumnDialect,
}

impl ColumnRef {
    /// Creates a lightweight `ColumnRef` for SQL rendering only.
    ///
    /// Only `table` and `name` are populated; metadata fields
    /// use empty defaults. Use a full struct literal for metadata-carrying refs.
    pub const fn sql(table: &'static str, name: &'static str) -> Self {
        Self {
            table,
            name,
            sql_type: "",
            not_null: false,
            primary_key: false,
            unique: false,
            has_default: false,
            dialect: ColumnDialect::SQLite {
                autoincrement: false,
            },
        }
    }
}

// ==================== SQLChunk ====================

/// A SQL chunk represents a part of an SQL statement.
///
/// Each variant has a clear semantic purpose:
/// - `Token` - SQL keywords and operators (SELECT, FROM, =, etc.)
/// - `Ident` - Quoted identifiers ("table_name", "column_name")
/// - `Raw` - Unquoted raw SQL text (function names, expressions)
/// - `Param` - Parameter placeholders with values
/// - `Table` - Table reference via `TableRef`
/// - `Column` - Column reference via `ColumnRef`
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
                let _ = buf.write_str(c.table);
                let _ = buf.write_str("\".\"");
                let _ = buf.write_str(c.name);
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
                .field(&format!("{}.{}", c.table, c.name))
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
