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
    #[must_use]
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

/// Packed column metadata flags for [`ColumnRef`].
///
/// Encodes the nullability, primary-key, unique, and has-default bits in a
/// single byte so that [`ColumnRef`] stays below the "too many bools" threshold
/// while keeping each bit independently addressable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ColumnFlags(u8);

impl ColumnFlags {
    /// Column is declared `NOT NULL`.
    pub const NOT_NULL: Self = Self(1 << 0);
    /// Column participates in the table's primary key.
    pub const PRIMARY_KEY: Self = Self(1 << 1);
    /// Column has a `UNIQUE` constraint.
    pub const UNIQUE: Self = Self(1 << 2);
    /// Column has a `DEFAULT` clause.
    pub const HAS_DEFAULT: Self = Self(1 << 3);

    /// Returns a flag set with no bits set.
    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Reconstructs a flag set from its raw byte representation.
    #[must_use]
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    /// Returns the raw byte representation.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Returns `true` when every bit in `other` is set in `self`.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Returns the union of two flag sets.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl core::ops::BitOr for ColumnFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

impl core::ops::BitOrAssign for ColumnFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.union(rhs);
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
    pub flags: ColumnFlags,

    // Dialect-specific
    pub dialect: ColumnDialect,
}

impl ColumnRef {
    /// Creates a lightweight `ColumnRef` for SQL rendering only.
    ///
    /// Only `table` and `name` are populated; metadata fields
    /// use empty defaults. Use a full struct literal for metadata-carrying refs.
    #[must_use]
    pub const fn sql(table: &'static str, name: &'static str) -> Self {
        Self {
            table,
            name,
            sql_type: "",
            flags: ColumnFlags::empty(),
            dialect: ColumnDialect::SQLite {
                autoincrement: false,
            },
        }
    }

    /// Returns `true` if this column is declared `NOT NULL`.
    #[must_use]
    pub const fn not_null(&self) -> bool {
        self.flags.contains(ColumnFlags::NOT_NULL)
    }

    /// Returns `true` if this column participates in the primary key.
    #[must_use]
    pub const fn primary_key(&self) -> bool {
        self.flags.contains(ColumnFlags::PRIMARY_KEY)
    }

    /// Returns `true` if this column has a `UNIQUE` constraint.
    #[must_use]
    pub const fn unique(&self) -> bool {
        self.flags.contains(ColumnFlags::UNIQUE)
    }

    /// Returns `true` if this column has a `DEFAULT` clause.
    #[must_use]
    pub const fn has_default(&self) -> bool {
        self.flags.contains(ColumnFlags::HAS_DEFAULT)
    }
}

// ==================== Identifier quoting ====================

/// Writes a SQL identifier enclosed in double quotes.
///
/// Any embedded `"` characters are doubled to prevent identifier-injection
/// (CWE-89). Both `PostgreSQL` and `SQLite` accept `"..."` as a delimited
/// identifier and treat `""` as an escaped double-quote character inside
/// such an identifier.
///
/// Fast path: identifiers with no embedded `"` are written with three calls
/// (open quote, name, close quote). Only identifiers containing a `"` take
/// the character-by-character escaping path.
#[inline]
pub fn write_quoted_ident(buf: &mut impl core::fmt::Write, name: &str) {
    let _ = buf.write_char('"');
    if name.contains('"') {
        for ch in name.chars() {
            if ch == '"' {
                let _ = buf.write_str("\"\"");
            } else {
                let _ = buf.write_char(ch);
            }
        }
    } else {
        let _ = buf.write_str(name);
    }
    let _ = buf.write_char('"');
}

// ==================== SQLChunk ====================

/// A SQL chunk represents a part of an SQL statement.
///
/// Each variant has a clear semantic purpose:
/// - `Token` - SQL keywords and operators (SELECT, FROM, =, etc.)
/// - `Ident` - Quoted identifiers ("`table_name`", "`column_name`")
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
    /// Renders as: "`table_name`"
    /// Column names used for SELECT * expansion.
    Table(TableRef),

    /// Column reference with static table and column names.
    /// Renders as: "`table_name"."column_name`"
    Column(ColumnRef),
}

impl<'a, V: SQLParam> SQLChunk<'a, V> {
    // ==================== const constructors ====================

    /// Creates a token chunk - const
    #[inline]
    #[must_use]
    pub const fn token(t: Token) -> Self {
        Self::Token(t)
    }

    /// Creates a quoted identifier from a static string - const
    #[inline]
    #[must_use]
    pub const fn ident_static(name: &'static str) -> Self {
        Self::Ident(Cow::Borrowed(name))
    }

    /// Creates raw SQL text from a static string - const
    #[inline]
    #[must_use]
    pub const fn raw_static(text: &'static str) -> Self {
        Self::Raw(Cow::Borrowed(text))
    }

    /// Creates a table chunk - const
    #[inline]
    #[must_use]
    pub const fn table(table: TableRef) -> Self {
        Self::Table(table)
    }

    /// Creates a column chunk - const
    #[inline]
    #[must_use]
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
    #[must_use]
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
                write_quoted_ident(buf, name);
            }
            SQLChunk::Raw(text) => {
                let _ = buf.write_str(text);
            }
            SQLChunk::Number(value) => {
                let _ = write!(buf, "{value}");
            }
            SQLChunk::Param(Param { placeholder, .. }) => {
                let _ = write!(buf, "{placeholder}");
            }
            SQLChunk::Table(t) => {
                write_quoted_ident(buf, t.name);
            }
            SQLChunk::Column(c) => {
                write_quoted_ident(buf, c.table);
                let _ = buf.write_char('.');
                write_quoted_ident(buf, c.name);
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

impl<V: SQLParam + core::fmt::Debug> core::fmt::Debug for SQLChunk<'_, V> {
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

impl<V: SQLParam> From<Token> for SQLChunk<'_, V> {
    #[inline]
    fn from(value: Token) -> Self {
        Self::Token(value)
    }
}

impl<V: SQLParam> From<TableRef> for SQLChunk<'_, V> {
    #[inline]
    fn from(value: TableRef) -> Self {
        Self::Table(value)
    }
}

impl<V: SQLParam> From<ColumnRef> for SQLChunk<'_, V> {
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
