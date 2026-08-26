//! Serializable MySQL DDL entities used by snapshots and migration planning.

use crate::alloc_prelude::*;

#[cfg(feature = "serde")]
use crate::serde_helpers::{cow_from_string, cow_option_from_string, cow_vec_from_strings};

/// Storage mode for a generated MySQL column.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum GeneratedType {
    Virtual,
    #[default]
    Stored,
}

/// Generated-column metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Generated {
    #[cfg_attr(
        feature = "serde",
        serde(rename = "as", deserialize_with = "cow_from_string")
    )]
    pub expression: Cow<'static, str>,
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub generation_type: GeneratedType,
}

impl Generated {
    #[must_use]
    pub fn stored(expression: impl Into<Cow<'static, str>>) -> Self {
        Self {
            expression: expression.into(),
            generation_type: GeneratedType::Stored,
        }
    }

    #[must_use]
    pub fn virtual_column(expression: impl Into<Cow<'static, str>>) -> Self {
        Self {
            expression: expression.into(),
            generation_type: GeneratedType::Virtual,
        }
    }
}

/// Values belonging to an inline `ENUM` or `SET` declaration.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct InlineEnum {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_vec_from_strings"))]
    pub values: Vec<Cow<'static, str>>,
}

impl InlineEnum {
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

/// Structured inline type data retained beside the rendered SQL type.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(tag = "kind", content = "definition", rename_all = "lowercase")
)]
pub enum InlineType {
    Enum(InlineEnum),
    Set(InlineEnum),
}

/// A table in the one selected MySQL database scope.
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
    pub temporary: bool,
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
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
    pub options: Vec<TableOption>,
}

impl Table {
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

/// A column and its complete MySQL definition.
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
    pub sql_type: Cow<'static, str>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub not_null: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub autoincrement: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub primary_key: bool,
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
    pub default: Option<Cow<'static, str>>,
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            deserialize_with = "cow_option_from_string",
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub on_update: Option<Cow<'static, str>>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub generated: Option<Generated>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
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

/// One index key part. Expressions are trusted schema SQL; plain column names
/// are distinguished by `is_expression`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct IndexColumn {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub expression: Cow<'static, str>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_expression: bool,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub length: Option<u32>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub ascending: Option<bool>,
}

impl IndexColumn {
    #[must_use]
    pub fn column(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            expression: name.into(),
            is_expression: false,
            length: None,
            ascending: None,
        }
    }

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

/// A currently unsupported table option. Keeping it typed prevents accepted
/// source metadata from being silently discarded by the planner.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TableOption {
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub value: Cow<'static, str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum IndexMethod {
    Btree,
    Hash,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum IndexAlgorithm {
    Default,
    Inplace,
    Copy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum IndexLock {
    Default,
    None,
    Shared,
    Exclusive,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
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
    pub columns: Vec<IndexColumn>,
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
    pub visible: Option<bool>,
}

impl Index {
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
pub enum ReferentialAction {
    #[cfg_attr(feature = "serde", serde(rename = "CASCADE"))]
    Cascade,
    #[cfg_attr(feature = "serde", serde(rename = "SET NULL"))]
    SetNull,
    #[cfg_attr(feature = "serde", serde(rename = "RESTRICT"))]
    Restrict,
    #[cfg_attr(feature = "serde", serde(rename = "NO ACTION"))]
    NoAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
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
    pub foreign_database: Option<Cow<'static, str>>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub foreign_table: Cow<'static, str>,
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_vec_from_strings"))]
    pub foreign_columns: Vec<Cow<'static, str>>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub on_delete: Option<ReferentialAction>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub on_update: Option<ReferentialAction>,
}

impl ForeignKey {
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
    pub enforced: Option<bool>,
}

impl CheckConstraint {
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
pub enum ViewAlgorithm {
    Undefined,
    Merge,
    Temptable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum ViewSqlSecurity {
    Definer,
    Invoker,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum ViewCheckOption {
    Cascaded,
    Local,
}

/// A MySQL view that can be faithfully recreated.
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
    pub is_existing: bool,
}

impl View {
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

/// Unified MySQL entity for the v6 `ddl` snapshot array.
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
    /// Explicit database carried by this entity, if any.
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
