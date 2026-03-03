//! FK-aware topological sort for determining table seeding order.
//!
//! Tables that are referenced by other tables (parents) must be seeded first.
//! Uses Kahn's algorithm for the topological sort.

use std::collections::{HashMap, HashSet, VecDeque};

use drizzle_core::TableRef;

fn parent_names<'a>(table: &'a TableRef, table_names: &HashSet<&'a str>) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for fk in table.foreign_keys {
        let name = fk.target_table;
        if table_names.contains(name) && name != table.name && seen.insert(name) {
            out.push(name);
        }
    }

    out
}

/// Compute seeding order for tables using topological sort.
///
/// Returns table names in order such that parent tables come before children.
/// If cycles exist, the cyclic tables are appended at the end (they require
/// a two-pass insert-then-update strategy).
pub fn seeding_order<'a>(tables: &[&'a TableRef]) -> Vec<&'a str> {
    let table_names: HashSet<&str> = tables.iter().map(|t| t.name).collect();

    // Build adjacency: table -> tables it depends on (via FK)
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

    for &table in tables {
        in_degree.entry(table.name).or_insert(0);
        for dep_name in parent_names(table, &table_names) {
            *in_degree.entry(table.name).or_insert(0) += 1;
            dependents.entry(dep_name).or_default().push(table.name);
        }
    }

    // Kahn's algorithm
    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(&name, _)| name)
        .collect();

    // Sort the initial queue for deterministic ordering
    let mut initial: Vec<&str> = queue.drain(..).collect();
    initial.sort();
    queue.extend(initial);

    let mut result: Vec<&str> = Vec::with_capacity(tables.len());

    while let Some(name) = queue.pop_front() {
        result.push(name);
        if let Some(deps) = dependents.get(name) {
            let mut next = Vec::new();
            for &dep in deps {
                let deg = in_degree.get_mut(dep).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    next.push(dep);
                }
            }
            // Sort for deterministic ordering
            next.sort();
            queue.extend(next);
        }
    }

    // Any remaining tables have cycles — append them at the end
    for &table in tables {
        if !result.contains(&table.name) {
            result.push(table.name);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use drizzle_core::{ColumnDialect, ForeignKeyRef, TableDialect};

    const fn test_table(name: &'static str, foreign_keys: &'static [ForeignKeyRef]) -> TableRef {
        TableRef {
            name,
            column_names: &[],
            schema: None,
            qualified_name: name,
            columns: &[],
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

    const fn test_fk(target: &'static str) -> ForeignKeyRef {
        ForeignKeyRef {
            target_table: target,
            source_columns: &[],
            target_columns: &[],
        }
    }

    #[test]
    fn linear_dependency() {
        static A: TableRef = test_table("a", &[]);
        static B_FKS: [ForeignKeyRef; 1] = [test_fk("a")];
        static B: TableRef = test_table("b", &B_FKS);
        static C_FKS: [ForeignKeyRef; 1] = [test_fk("b")];
        static C: TableRef = test_table("c", &C_FKS);

        let tables: Vec<&TableRef> = vec![&C, &B, &A];
        let order = seeding_order(&tables);
        assert_eq!(order, vec!["a", "b", "c"]);
    }

    #[test]
    fn no_dependencies() {
        static A: TableRef = test_table("a", &[]);
        static B: TableRef = test_table("b", &[]);
        static C: TableRef = test_table("c", &[]);

        let tables: Vec<&TableRef> = vec![&C, &B, &A];
        let order = seeding_order(&tables);
        // Alphabetically sorted when no dependencies
        assert_eq!(order, vec!["a", "b", "c"]);
    }

    #[test]
    fn diamond_dependency() {
        // D depends on B and C, both depend on A
        //     A
        //    / \
        //   B   C
        //    \ /
        //     D
        static A: TableRef = test_table("a", &[]);
        static B_FKS: [ForeignKeyRef; 1] = [test_fk("a")];
        static B: TableRef = test_table("b", &B_FKS);
        static C_FKS: [ForeignKeyRef; 1] = [test_fk("a")];
        static C: TableRef = test_table("c", &C_FKS);
        static D_FKS: [ForeignKeyRef; 2] = [test_fk("b"), test_fk("c")];
        static D: TableRef = test_table("d", &D_FKS);

        let tables: Vec<&TableRef> = vec![&D, &C, &A, &B];
        let order = seeding_order(&tables);
        // A must come first, then B and C (alphabetical tie-break), then D
        assert_eq!(order, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn cycle_detection() {
        // A -> B -> C -> A (cycle)
        // Cycles should still produce all tables (appended at end)
        static A_FKS: [ForeignKeyRef; 1] = [test_fk("c")];
        static A: TableRef = test_table("a", &A_FKS);
        static B_FKS: [ForeignKeyRef; 1] = [test_fk("a")];
        static B: TableRef = test_table("b", &B_FKS);
        static C_FKS: [ForeignKeyRef; 1] = [test_fk("b")];
        static C: TableRef = test_table("c", &C_FKS);

        let tables: Vec<&TableRef> = vec![&A, &B, &C];
        let order = seeding_order(&tables);
        // All tables must be present even with cycles
        assert_eq!(order.len(), 3);
        assert!(order.contains(&"a"));
        assert!(order.contains(&"b"));
        assert!(order.contains(&"c"));
    }

    #[test]
    fn self_reference_ignored() {
        // A table that references itself (self-referential FK)
        static A_FKS: [ForeignKeyRef; 1] = [test_fk("a")];
        static A: TableRef = test_table("a", &A_FKS);

        let tables: Vec<&TableRef> = vec![&A];
        let order = seeding_order(&tables);
        // Self-references are filtered out (dep_name != table.name)
        assert_eq!(order, vec!["a"]);
    }

    #[test]
    fn single_table() {
        static A: TableRef = test_table("a", &[]);

        let tables: Vec<&TableRef> = vec![&A];
        let order = seeding_order(&tables);
        assert_eq!(order, vec!["a"]);
    }

    #[test]
    fn dependency_outside_table_set_ignored() {
        // B depends on X, but X is not in our table set
        static A: TableRef = test_table("a", &[]);
        static B_FKS: [ForeignKeyRef; 1] = [test_fk("x")];
        static B: TableRef = test_table("b", &B_FKS);

        // Only pass A and B, not X
        let tables: Vec<&TableRef> = vec![&B, &A];
        let order = seeding_order(&tables);
        // X is not in the set, so B's dependency on X is ignored
        assert_eq!(order, vec!["a", "b"]);
    }

    #[test]
    fn wide_fan_out() {
        // Parent with many children
        static PARENT: TableRef = test_table("parent", &[]);
        static C1_FKS: [ForeignKeyRef; 1] = [test_fk("parent")];
        static C1: TableRef = test_table("child_1", &C1_FKS);
        static C2_FKS: [ForeignKeyRef; 1] = [test_fk("parent")];
        static C2: TableRef = test_table("child_2", &C2_FKS);
        static C3_FKS: [ForeignKeyRef; 1] = [test_fk("parent")];
        static C3: TableRef = test_table("child_3", &C3_FKS);
        static C4_FKS: [ForeignKeyRef; 1] = [test_fk("parent")];
        static C4: TableRef = test_table("child_4", &C4_FKS);
        static C5_FKS: [ForeignKeyRef; 1] = [test_fk("parent")];
        static C5: TableRef = test_table("child_5", &C5_FKS);

        let tables: Vec<&TableRef> = vec![&C5, &C3, &C1, &PARENT, &C4, &C2];
        let order = seeding_order(&tables);
        // Parent first, then children in alphabetical order
        assert_eq!(order[0], "parent");
        assert_eq!(
            &order[1..],
            &["child_1", "child_2", "child_3", "child_4", "child_5"]
        );
    }

    #[test]
    fn multi_level_chain() {
        // A -> B -> C -> D -> E (5 levels deep)
        static A: TableRef = test_table("a", &[]);
        static B_FKS: [ForeignKeyRef; 1] = [test_fk("a")];
        static B: TableRef = test_table("b", &B_FKS);
        static C_FKS: [ForeignKeyRef; 1] = [test_fk("b")];
        static C: TableRef = test_table("c", &C_FKS);
        static D_FKS: [ForeignKeyRef; 1] = [test_fk("c")];
        static D: TableRef = test_table("d", &D_FKS);
        static E_FKS: [ForeignKeyRef; 1] = [test_fk("d")];
        static E: TableRef = test_table("e", &E_FKS);

        // Pass in reverse order
        let tables: Vec<&TableRef> = vec![&E, &D, &C, &B, &A];
        let order = seeding_order(&tables);
        assert_eq!(order, vec!["a", "b", "c", "d", "e"]);
    }
}
