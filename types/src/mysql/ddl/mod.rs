//! MySQL schema objects as stored in migration snapshots.
//!
//! The migration tooling builds these from your schema or by introspecting a
//! live database, saves them in snapshot files, and diffs two sets to plan a
//! migration. A full schema is a list of [`MySQLEntity`] values.
//!
//! Every entity has a `database` field. `None` means the connection's current
//! database. SQL fragments (`sql_type`, defaults, expressions, view
//! definitions) are stored as written and are trusted schema SQL.

use crate::alloc_prelude::*;

#[cfg(feature = "serde")]
use crate::serde_helpers::{cow_from_string, cow_option_from_string, cow_vec_from_strings};

/// Storage mode for a generated MySQL column.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum GeneratedType {
    /// `VIRTUAL`: computed when read.
    Virtual,
    /// `STORED`: computed on write and stored. The default.
    #[default]
    Stored,
}

/// The `GENERATED ALWAYS AS (...)` part of a column definition.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Generated {
    #[cfg_attr(
        feature = "serde",
        serde(rename = "as", deserialize_with = "cow_from_string")
    )]
    /// SQL expression that computes the value (serialized as `as`).
    pub expression: Cow<'static, str>,
    /// Whether the value is stored or virtual (serialized as `type`).
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub generation_type: GeneratedType,
}

impl Generated {
    /// Creates a `STORED` generated column from a SQL expression.
    #[must_use]
    pub fn stored(expression: impl Into<Cow<'static, str>>) -> Self {
        Self {
            expression: expression.into(),
            generation_type: GeneratedType::Stored,
        }
    }

    /// Creates a `VIRTUAL` generated column from a SQL expression.
    #[must_use]
    pub fn virtual_column(expression: impl Into<Cow<'static, str>>) -> Self {
        Self {
            expression: expression.into(),
            generation_type: GeneratedType::Virtual,
        }
    }
}

/// The allowed values of an inline `ENUM(...)` or `SET(...)` column type, in
/// declaration order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct InlineEnum {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_vec_from_strings"))]
    pub values: Vec<Cow<'static, str>>,
}

impl InlineEnum {
    /// Creates the value list from any iterable of strings.
    #[must_use]
    pub fn new(values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            values: values
                .into_iter()
                .map(|value| Cow::Owned(value.into()))
                .collect(),
        }
    }
}

/// The values of an inline `ENUM` or `SET` column, kept beside its rendered
/// `sql_type` so a diff can compare them without parsing SQL.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(tag = "kind", content = "definition", rename_all = "lowercase")
)]
pub enum InlineType {
    /// `ENUM('a', 'b', ...)`: exactly one of the values.
    Enum(InlineEnum),
    /// `SET('a', 'b', ...)`: any combination of the values.
    Set(InlineEnum),
}

/// A table, with its table options. Columns, keys and indexes are separate
/// entities that name the table.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Table {
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub database: Option<Cow<'static, str>>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "core::ops::Not::not")
    )]
    /// `CREATE TEMPORARY TABLE`.
    pub temporary: bool,
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    /// Storage engine, such as `InnoDB`.
    pub engine: Option<Cow<'static, str>>,
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub charset: Option<Cow<'static, str>>,
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub collation: Option<Cow<'static, str>>,
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub comment: Option<Cow<'static, str>>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    /// Other table options, which drizzle keeps but cannot change.
    pub options: Vec<TableOption>,
}

impl Table {
    /// Creates a table in the current database with no options set.
    #[must_use]
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            database: None,
            name: name.into(),
            temporary: false,
            engine: None,
            charset: None,
            collation: None,
            comment: None,
            options: Vec::new(),
        }
    }
}

/// A table column and its full MySQL definition.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Column {
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub database: Option<Cow<'static, str>>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub table: Cow<'static, str>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,
    #[cfg_attr(
        feature = "serde",
        serde(rename = "type", deserialize_with = "cow_from_string")
    )]
    /// Column type as written in DDL, such as `varchar(255)` or
    /// `int unsigned` (serialized as `type`).
    pub sql_type: Cow<'static, str>,
    /// `NOT NULL`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub not_null: bool,
    /// `AUTO_INCREMENT`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub autoincrement: bool,
    /// Whether the column is part of the primary key.
    #[cfg_attr(feature = "serde", serde(default))]
    pub primary_key: bool,
    /// Whether the column has a single-column unique constraint.
    #[cfg_attr(feature = "serde", serde(default))]
    pub unique: bool,
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    /// `DEFAULT` value, as SQL.
    pub default: Option<Cow<'static, str>>,
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    /// `ON UPDATE` value, as SQL, such as `CURRENT_TIMESTAMP`.
    pub on_update: Option<Cow<'static, str>>,
    /// `GENERATED ALWAYS AS (...)`, for a generated column.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub generated: Option<Generated>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    /// Values of an `ENUM` or `SET` column.
    pub inline_type: Option<InlineType>,
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub charset: Option<Cow<'static, str>>,
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub collation: Option<Cow<'static, str>>,
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub comment: Option<Cow<'static, str>>,
}

impl Column {
    /// Creates a nullable column with no default, constraints or options.
    #[must_use]
    pub fn new(
        table: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
        sql_type: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            database: None,
            table: table.into(),
            name: name.into(),
            sql_type: sql_type.into(),
            not_null: false,
            autoincrement: false,
            primary_key: false,
            unique: false,
            default: None,
            on_update: None,
            generated: None,
            inline_type: None,
            charset: None,
            collation: None,
            comment: None,
        }
    }
}

/// One key part of an index: a column name or a SQL expression.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct IndexColumn {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    /// The column name, or the SQL expression when `is_expression` is set.
    pub expression: Cow<'static, str>,
    /// Whether `expression` is a SQL expression rather than a column name.
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_expression: bool,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    /// Prefix length, as in `name(10)`.
    pub length: Option<u32>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    /// Sort order: `Some(true)` for `ASC`, `Some(false)` for `DESC`, `None`
    /// when not written (ascending).
    pub ascending: Option<bool>,
}

impl IndexColumn {
    /// Creates a key part for a column.
    #[must_use]
    pub fn column(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            expression: name.into(),
            is_expression: false,
            length: None,
            ascending: None,
        }
    }

    /// Creates a key part for a SQL expression.
    #[must_use]
    pub fn expression(sql: impl Into<Cow<'static, str>>) -> Self {
        Self {
            expression: sql.into(),
            is_expression: true,
            length: None,
            ascending: None,
        }
    }
}

/// A table option drizzle does not model, kept as a name and value so it is
/// not silently dropped. The planner reports changes to these as unsupported.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TableOption {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    /// Option name.
    pub name: Cow<'static, str>,
    /// Option value, as SQL.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub value: Cow<'static, str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
/// Index type, from `USING BTREE` or `USING HASH`.
pub enum IndexMethod {
    /// `BTREE`.
    Btree,
    /// `HASH`.
    Hash,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
/// `ALGORITHM` option used when creating an index.
pub enum IndexAlgorithm {
    /// `ALGORITHM = DEFAULT`.
    Default,
    /// `ALGORITHM = INPLACE`.
    Inplace,
    /// `ALGORITHM = COPY`.
    Copy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
/// `LOCK` option used when creating an index.
pub enum IndexLock {
    /// `LOCK = DEFAULT`.
    Default,
    /// `LOCK = NONE`.
    None,
    /// `LOCK = SHARED`.
    Shared,
    /// `LOCK = EXCLUSIVE`.
    Exclusive,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
/// An index on a table.
pub struct Index {
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub database: Option<Cow<'static, str>>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub table: Cow<'static, str>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,
    /// Key parts, in order.
    pub columns: Vec<IndexColumn>,
    /// `UNIQUE INDEX`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub unique: bool,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub using: Option<IndexMethod>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub algorithm: Option<IndexAlgorithm>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub lock: Option<IndexLock>,
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub comment: Option<Cow<'static, str>>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    /// `VISIBLE` (`Some(true)`) or `INVISIBLE` (`Some(false)`); `None`
    /// means not written (visible).
    pub visible: Option<bool>,
}

impl Index {
    /// Creates a non-unique index with no options.
    #[must_use]
    pub fn new(
        table: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
        columns: Vec<IndexColumn>,
    ) -> Self {
        Self {
            database: None,
            table: table.into(),
            name: name.into(),
            columns,
            unique: false,
            using: None,
            algorithm: None,
            lock: None,
            comment: None,
            visible: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
/// A table's primary key.
pub struct PrimaryKey {
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub database: Option<Cow<'static, str>>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub table: Cow<'static, str>,
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub name: Option<Cow<'static, str>>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_vec_from_strings"))]
    pub columns: Vec<Cow<'static, str>>,
}

impl PrimaryKey {
    /// Creates a primary key over `columns`, named `PRIMARY` (MySQL's fixed
    /// name for primary keys).
    #[must_use]
    pub fn new(
        table: impl Into<Cow<'static, str>>,
        columns: impl IntoIterator<Item = impl Into<Cow<'static, str>>>,
    ) -> Self {
        Self {
            database: None,
            table: table.into(),
            name: Some(Cow::Borrowed("PRIMARY")),
            columns: columns.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
/// A named unique constraint.
pub struct UniqueConstraint {
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub database: Option<Cow<'static, str>>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub table: Cow<'static, str>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_vec_from_strings"))]
    pub columns: Vec<Cow<'static, str>>,
}

impl UniqueConstraint {
    /// Creates a unique constraint over `columns`.
    #[must_use]
    pub fn new(
        table: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
        columns: impl IntoIterator<Item = impl Into<Cow<'static, str>>>,
    ) -> Self {
        Self {
            database: None,
            table: table.into(),
            name: name.into(),
            columns: columns.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Foreign key `ON DELETE` / `ON UPDATE` action. Serialized as the SQL
/// keywords (`"CASCADE"`, `"SET NULL"`, ...).
pub enum ReferentialAction {
    /// `CASCADE`.
    #[cfg_attr(feature = "serde", serde(rename = "CASCADE"))]
    Cascade,
    /// `SET NULL`.
    #[cfg_attr(feature = "serde", serde(rename = "SET NULL"))]
    SetNull,
    /// `RESTRICT`.
    #[cfg_attr(feature = "serde", serde(rename = "RESTRICT"))]
    Restrict,
    /// `NO ACTION`.
    #[cfg_attr(feature = "serde", serde(rename = "NO ACTION"))]
    NoAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
/// A foreign key from `table(columns)` to `foreign_table(foreign_columns)`.
pub struct ForeignKey {
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub database: Option<Cow<'static, str>>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub table: Cow<'static, str>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_vec_from_strings"))]
    pub columns: Vec<Cow<'static, str>>,
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    /// Database of the referenced table; `None` for the current database.
    pub foreign_database: Option<Cow<'static, str>>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub foreign_table: Cow<'static, str>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_vec_from_strings"))]
    pub foreign_columns: Vec<Cow<'static, str>>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    /// `ON DELETE` action; `None` when not written.
    pub on_delete: Option<ReferentialAction>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    /// `ON UPDATE` action; `None` when not written.
    pub on_update: Option<ReferentialAction>,
}

impl ForeignKey {
    /// Creates a foreign key with no `ON DELETE` / `ON UPDATE` actions.
    #[must_use]
    pub fn new(
        table: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
        columns: impl IntoIterator<Item = impl Into<Cow<'static, str>>>,
        foreign_table: impl Into<Cow<'static, str>>,
        foreign_columns: impl IntoIterator<Item = impl Into<Cow<'static, str>>>,
    ) -> Self {
        Self {
            database: None,
            table: table.into(),
            name: name.into(),
            columns: columns.into_iter().map(Into::into).collect(),
            foreign_database: None,
            foreign_table: foreign_table.into(),
            foreign_columns: foreign_columns.into_iter().map(Into::into).collect(),
            on_delete: None,
            on_update: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
/// A named `CHECK` constraint.
pub struct CheckConstraint {
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub database: Option<Cow<'static, str>>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub table: Cow<'static, str>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub expression: Cow<'static, str>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    /// `ENFORCED` (`Some(true)`) or `NOT ENFORCED` (`Some(false)`); `None`
    /// means not written (enforced).
    pub enforced: Option<bool>,
}

impl CheckConstraint {
    /// Creates a check constraint from a SQL boolean expression.
    #[must_use]
    pub fn new(
        table: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
        expression: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            database: None,
            table: table.into(),
            name: name.into(),
            expression: expression.into(),
            enforced: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
/// View `ALGORITHM`.
pub enum ViewAlgorithm {
    /// `UNDEFINED`: MySQL chooses.
    Undefined,
    /// `MERGE`.
    Merge,
    /// `TEMPTABLE`.
    Temptable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
/// View `SQL SECURITY`: whose privileges the view runs with.
pub enum ViewSqlSecurity {
    /// `DEFINER`.
    Definer,
    /// `INVOKER`.
    Invoker,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
/// View `WITH ... CHECK OPTION`.
pub enum ViewCheckOption {
    /// `WITH CASCADED CHECK OPTION`.
    Cascaded,
    /// `WITH LOCAL CHECK OPTION`.
    Local,
}

/// A view.
///
/// A view with `is_existing` set is managed outside drizzle: it is referenced
/// but never created or dropped, and needs no `definition`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct View {
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub database: Option<Cow<'static, str>>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    /// The view's `SELECT` statement. Required unless `is_existing` is set.
    pub definition: Option<Cow<'static, str>>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub algorithm: Option<ViewAlgorithm>,
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    /// `DEFINER` account, such as `` `app`@`%` ``.
    pub definer: Option<Cow<'static, str>>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub sql_security: Option<ViewSqlSecurity>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub check_option: Option<ViewCheckOption>,
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub charset: Option<Cow<'static, str>>,
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub collation: Option<Cow<'static, str>>,
    #[cfg_attr(feature = "serde", serde(default))]
    /// Whether the view exists outside drizzle's control.
    pub is_existing: bool,
}

impl View {
    /// Creates a view from its `SELECT` statement, with no options.
    #[must_use]
    pub fn new(
        name: impl Into<Cow<'static, str>>,
        definition: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            database: None,
            name: name.into(),
            definition: Some(definition.into()),
            algorithm: None,
            definer: None,
            sql_security: None,
            check_option: None,
            charset: None,
            collation: None,
            is_existing: false,
        }
    }
}

/// Any MySQL schema object: one element of a snapshot's `ddl` array.
///
/// With `serde`, the variant is stored in an `entityType` field (`"tables"`,
/// `"columns"`, `"indexes"`, `"pks"`, `"uniques"`, `"fks"`, `"checks"`,
/// `"views"`).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "entityType"))]
pub enum MySQLEntity {
    #[cfg_attr(feature = "serde", serde(rename = "tables"))]
    Table(Table),
    #[cfg_attr(feature = "serde", serde(rename = "columns"))]
    Column(Column),
    #[cfg_attr(feature = "serde", serde(rename = "indexes"))]
    Index(Index),
    #[cfg_attr(feature = "serde", serde(rename = "pks"))]
    PrimaryKey(PrimaryKey),
    #[cfg_attr(feature = "serde", serde(rename = "uniques"))]
    UniqueConstraint(UniqueConstraint),
    #[cfg_attr(feature = "serde", serde(rename = "fks"))]
    ForeignKey(ForeignKey),
    #[cfg_attr(feature = "serde", serde(rename = "checks"))]
    CheckConstraint(CheckConstraint),
    #[cfg_attr(feature = "serde", serde(rename = "views"))]
    View(View),
}

impl MySQLEntity {
    /// Returns the entity's database, or `None` for the current database.
    #[must_use]
    pub fn database(&self) -> Option<&str> {
        match self {
            Self::Table(entity) => entity.database.as_deref(),
            Self::Column(entity) => entity.database.as_deref(),
            Self::Index(entity) => entity.database.as_deref(),
            Self::PrimaryKey(entity) => entity.database.as_deref(),
            Self::UniqueConstraint(entity) => entity.database.as_deref(),
            Self::ForeignKey(entity) => entity.database.as_deref(),
            Self::CheckConstraint(entity) => entity.database.as_deref(),
            Self::View(entity) => entity.database.as_deref(),
        }
    }
}
