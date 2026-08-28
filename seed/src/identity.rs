use core::fmt;

use drizzle_core::{ForeignKeyRef, SQLTableInfo, TableRef};

/// Namespace-aware identity used by the seed planner.
///
/// Database/schema names are part of table identity. Keeping this private lets
/// the public builder stay typed while preventing two equally named tables in
/// different namespaces from sharing counts, generators, or generated keys.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct TableId {
    pub(crate) schema: Option<&'static str>,
    pub(crate) name: &'static str,
}

impl TableId {
    pub(crate) fn new(schema: Option<&'static str>, name: &'static str) -> Self {
        Self {
            schema: normalize_schema(schema),
            name,
        }
    }

    pub(crate) fn from_ref(table: &TableRef) -> Self {
        Self::new(table.schema, table.name)
    }

    pub(crate) fn from_info(table: &(impl SQLTableInfo + ?Sized)) -> Self {
        Self::new(table.schema(), table.name())
    }

    /// Resolve a foreign-key target. Macro metadata uses an empty target
    /// schema for an unqualified reference, which means the source namespace.
    pub(crate) fn foreign_target(source: &TableRef, foreign_key: &ForeignKeyRef) -> Self {
        let schema = if foreign_key.target_schema.is_empty() {
            source.schema
        } else {
            Some(foreign_key.target_schema)
        };
        Self::new(schema, foreign_key.target_table)
    }
}

impl fmt::Display for TableId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(schema) = self.schema {
            write!(formatter, "{schema}.{}", self.name)
        } else {
            formatter.write_str(self.name)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct ColumnId {
    pub(crate) table: TableId,
    pub(crate) name: &'static str,
}

impl ColumnId {
    pub(crate) const fn new(table: TableId, name: &'static str) -> Self {
        Self { table, name }
    }

    pub(crate) fn from_info(column: &impl drizzle_core::SQLColumnInfo) -> Self {
        Self::new(TableId::from_info(column.table()), column.name())
    }
}

fn normalize_schema(schema: Option<&'static str>) -> Option<&'static str> {
    match schema {
        Some("") | None => None,
        Some(schema) => Some(schema),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use drizzle_core::TableDialect;

    const SOURCE: TableRef = TableRef {
        name: "children",
        column_names: &[],
        schema: Some("tenant_a"),
        qualified_name: "tenant_a.children",
        columns: &[],
        primary_key: None,
        foreign_keys: &[],
        constraints: &[],
        dependency_names: &[],
        dialect: TableDialect::MySQL {
            is_temporary: false,
            engine: None,
            charset: None,
            collate: None,
            comment: None,
        },
    };

    const fn foreign_key(target_schema: &'static str) -> ForeignKeyRef {
        ForeignKeyRef {
            name: "fk",
            name_explicit: false,
            target_table: "parents",
            target_schema,
            source_columns: &[],
            target_columns: &[],
            on_delete: None,
            on_update: None,
            deferrable: false,
            initially_deferred: false,
        }
    }

    #[test]
    fn unqualified_foreign_keys_inherit_the_source_namespace() {
        assert_eq!(
            TableId::foreign_target(&SOURCE, &foreign_key("")),
            TableId::new(Some("tenant_a"), "parents")
        );
    }

    #[test]
    fn qualified_foreign_keys_keep_the_target_namespace() {
        assert_eq!(
            TableId::foreign_target(&SOURCE, &foreign_key("tenant_b")),
            TableId::new(Some("tenant_b"), "parents")
        );
    }
}
