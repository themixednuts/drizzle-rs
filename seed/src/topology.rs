//! FK-aware table ordering for deterministic seed plans.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use drizzle_core::{ForeignKeyRef, TableRef};

use crate::identity::TableId;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TopologyError {
    pub(crate) tables: Vec<TableId>,
}

fn parent_ids(table: &TableRef, table_ids: &HashSet<TableId>) -> Vec<TableId> {
    let mut parents = BTreeSet::new();
    for foreign_key in table.foreign_keys {
        let parent = TableId::foreign_target(table, foreign_key);
        if is_nullable_self_reference(table, foreign_key) {
            continue;
        }
        if table_ids.contains(&parent) {
            parents.insert(parent);
        }
    }
    parents.into_iter().collect()
}

fn is_nullable_self_reference(table: &TableRef, foreign_key: &ForeignKeyRef) -> bool {
    TableId::foreign_target(table, foreign_key) == TableId::from_ref(table)
        && !foreign_key.source_columns.is_empty()
        && foreign_key.source_columns.iter().all(|source| {
            table
                .columns
                .iter()
                .find(|column| column.name == *source)
                .is_some()
        })
        && foreign_key.source_columns.iter().any(|source| {
            table
                .columns
                .iter()
                .find(|column| column.name == *source)
                .is_some_and(|column| !column.not_null())
        })
}

/// Nullable self-referencing columns that must be cleared before a reset
/// deletes the table's existing rows.
pub(crate) fn nullable_self_reference_columns(table: &TableRef) -> Vec<&'static str> {
    let mut columns = BTreeSet::new();
    for foreign_key in table.foreign_keys {
        if is_nullable_self_reference(table, foreign_key) {
            columns.extend(foreign_key.source_columns.iter().copied().filter(|source| {
                table
                    .columns
                    .iter()
                    .find(|column| column.name == *source)
                    .is_some_and(|column| !column.not_null())
            }));
        }
    }
    columns.into_iter().collect()
}

/// Return parents before children, with namespace-aware deterministic ties.
///
/// Cycles are rejected instead of silently producing invalid inserts. None of
/// the supported dialects has a portable way to satisfy a non-null cycle in a
/// single insert pass, and MySQL does not support deferred constraints.
pub(crate) fn seeding_order(tables: &[&TableRef]) -> Result<Vec<TableId>, TopologyError> {
    let table_ids: HashSet<_> = tables
        .iter()
        .map(|table| TableId::from_ref(table))
        .collect();
    let mut in_degree: HashMap<TableId, usize> = HashMap::new();
    let mut dependents: HashMap<TableId, Vec<TableId>> = HashMap::new();

    for table in tables {
        let child = TableId::from_ref(table);
        in_degree.entry(child).or_insert(0);
        for parent in parent_ids(table, &table_ids) {
            *in_degree.entry(child).or_insert(0) += 1;
            dependents.entry(parent).or_default().push(child);
        }
    }

    for children in dependents.values_mut() {
        children.sort_unstable();
        children.dedup();
    }

    let initial: BTreeSet<_> = in_degree
        .iter()
        .filter_map(|(table, degree)| (*degree == 0).then_some(*table))
        .collect();
    let mut queue: VecDeque<_> = initial.into_iter().collect();
    let mut result = Vec::with_capacity(tables.len());

    while let Some(table) = queue.pop_front() {
        result.push(table);
        if let Some(children) = dependents.get(&table) {
            let mut ready = BTreeSet::new();
            for child in children {
                let degree = in_degree
                    .get_mut(child)
                    .expect("every dependent table has an in-degree entry");
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(*child);
                }
            }
            queue.extend(ready);
        }
    }

    if result.len() == tables.len() {
        return Ok(result);
    }

    let emitted: HashSet<_> = result.into_iter().collect();
    let mut cyclic: Vec<_> = table_ids
        .into_iter()
        .filter(|table| !emitted.contains(table))
        .collect();
    cyclic.sort_unstable();
    Err(TopologyError { tables: cyclic })
}

#[cfg(test)]
mod tests {
    use super::*;
    use drizzle_core::{ColumnDialect, ColumnFlags, ColumnRef, ForeignKeyRef, TableDialect};

    const fn table(
        schema: Option<&'static str>,
        name: &'static str,
        foreign_keys: &'static [ForeignKeyRef],
    ) -> TableRef {
        table_with_columns(schema, name, &[], foreign_keys)
    }

    const fn table_with_columns(
        schema: Option<&'static str>,
        name: &'static str,
        columns: &'static [ColumnRef],
        foreign_keys: &'static [ForeignKeyRef],
    ) -> TableRef {
        TableRef {
            name,
            column_names: &[],
            schema,
            qualified_name: name,
            columns,
            primary_key: None,
            foreign_keys,
            constraints: &[],
            dependency_names: &[],
            dialect: TableDialect::SQLite {
                without_rowid: false,
                strict: false,
            },
        }
    }

    const fn column(name: &'static str, not_null: bool) -> ColumnRef {
        ColumnRef {
            table: "nodes",
            name,
            sql_type: "INTEGER",
            flags: if not_null {
                ColumnFlags::NOT_NULL
            } else {
                ColumnFlags::empty()
            },
            dialect: ColumnDialect::SQLite {
                autoincrement: false,
                default: None,
                generated_expression: None,
                generated_stored: false,
                collate: None,
            },
        }
    }

    const fn foreign_key(target_schema: &'static str, target_table: &'static str) -> ForeignKeyRef {
        ForeignKeyRef {
            name: "fk",
            name_explicit: false,
            target_table,
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
    fn orders_a_dependency_chain() {
        static PARENT: TableRef = table(None, "parent", &[]);
        static CHILD_FKS: [ForeignKeyRef; 1] = [foreign_key("", "parent")];
        static CHILD: TableRef = table(None, "child", &CHILD_FKS);
        assert_eq!(
            seeding_order(&[&CHILD, &PARENT]).unwrap(),
            [TableId::new(None, "parent"), TableId::new(None, "child")]
        );
    }

    #[test]
    fn treats_namespaces_as_part_of_identity() {
        static A: TableRef = table(Some("a"), "users", &[]);
        static B: TableRef = table(Some("b"), "users", &[]);
        assert_eq!(
            seeding_order(&[&B, &A]).unwrap(),
            [
                TableId::new(Some("a"), "users"),
                TableId::new(Some("b"), "users")
            ]
        );
    }

    #[test]
    fn unqualified_foreign_key_uses_source_namespace() {
        static PARENT: TableRef = table(Some("tenant"), "parent", &[]);
        static CHILD_FKS: [ForeignKeyRef; 1] = [foreign_key("", "parent")];
        static CHILD: TableRef = table(Some("tenant"), "child", &CHILD_FKS);
        assert_eq!(
            seeding_order(&[&CHILD, &PARENT]).unwrap(),
            [
                TableId::new(Some("tenant"), "parent"),
                TableId::new(Some("tenant"), "child")
            ]
        );
    }

    #[test]
    fn partially_nullable_composite_self_reference_is_breakable() {
        static COLUMNS: [ColumnRef; 2] = [column("tenant_id", true), column("parent_id", false)];
        static SOURCE_COLUMNS: [&str; 2] = ["tenant_id", "parent_id"];
        static TARGET_COLUMNS: [&str; 2] = ["tenant_id", "parent_id"];
        static FOREIGN_KEYS: [ForeignKeyRef; 1] = [ForeignKeyRef {
            name: "fk_nodes_parent",
            name_explicit: false,
            target_table: "nodes",
            target_schema: "",
            source_columns: &SOURCE_COLUMNS,
            target_columns: &TARGET_COLUMNS,
            on_delete: None,
            on_update: None,
            deferrable: false,
            initially_deferred: false,
        }];
        static NODES: TableRef = table_with_columns(None, "nodes", &COLUMNS, &FOREIGN_KEYS);

        assert_eq!(
            seeding_order(&[&NODES]).unwrap(),
            [TableId::new(None, "nodes")]
        );
        assert_eq!(nullable_self_reference_columns(&NODES), ["parent_id"]);
    }

    #[test]
    fn rejects_cycles_including_self_references() {
        static SELF_FKS: [ForeignKeyRef; 1] = [foreign_key("", "node")];
        static NODE: TableRef = table(None, "node", &SELF_FKS);
        assert_eq!(
            seeding_order(&[&NODE]).unwrap_err(),
            TopologyError {
                tables: vec![TableId::new(None, "node")]
            }
        );
    }

    #[test]
    fn ignores_dependencies_outside_the_active_set() {
        static CHILD_FKS: [ForeignKeyRef; 1] = [foreign_key("", "external")];
        static CHILD: TableRef = table(None, "child", &CHILD_FKS);
        assert_eq!(
            seeding_order(&[&CHILD]).unwrap(),
            [TableId::new(None, "child")]
        );
    }
}
