//! FK-aware topological sort for determining table seeding order.
//!
//! Tables that are referenced by other tables (parents) must be seeded first.
//! Uses Kahn's algorithm for the topological sort.

use std::collections::{HashMap, HashSet, VecDeque};

use drizzle_core::SQLTableInfo;

fn parent_names<'a>(table: &'a dyn SQLTableInfo, table_names: &HashSet<&'a str>) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for fk in table.foreign_keys() {
        let name = fk.target_table().name();
        if table_names.contains(name) && name != table.name() && seen.insert(name) {
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
pub fn seeding_order(tables: &[&dyn SQLTableInfo]) -> Vec<String> {
    let table_names: HashSet<&str> = tables.iter().map(|t| t.name()).collect();

    // Build adjacency: table -> tables it depends on (via FK)
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

    for &table in tables {
        in_degree.entry(table.name()).or_insert(0);
        for dep_name in parent_names(table, &table_names) {
            *in_degree.entry(table.name()).or_insert(0) += 1;
            dependents.entry(dep_name).or_default().push(table.name());
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

    let mut result: Vec<String> = Vec::with_capacity(tables.len());

    while let Some(name) = queue.pop_front() {
        result.push(name.to_string());
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
        if !result.iter().any(|r| r == table.name()) {
            result.push(table.name().to_string());
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use drizzle_core::SQLForeignKeyInfo;

    struct TestTable {
        name: &'static str,
        fks: &'static [&'static dyn SQLForeignKeyInfo],
    }

    struct TestForeignKey {
        target: &'static dyn SQLTableInfo,
    }

    impl SQLForeignKeyInfo for TestForeignKey {
        fn source_table(&self) -> &'static dyn SQLTableInfo {
            panic!("test FK source_table is not used")
        }

        fn target_table(&self) -> &'static dyn SQLTableInfo {
            self.target
        }

        fn source_columns(&self) -> &'static [&'static str] {
            &[]
        }

        fn target_columns(&self) -> &'static [&'static str] {
            &[]
        }
    }

    impl SQLTableInfo for TestTable {
        fn name(&self) -> &str {
            self.name
        }
        fn columns(&self) -> &'static [&'static dyn drizzle_core::SQLColumnInfo] {
            &[]
        }
        fn foreign_keys(&self) -> &'static [&'static dyn SQLForeignKeyInfo] {
            self.fks
        }
        fn dependencies(&self) -> &'static [&'static dyn SQLTableInfo] {
            &[]
        }
    }

    #[test]
    fn linear_dependency() {
        static A: TestTable = TestTable {
            name: "a",
            fks: &[],
        };
        static B_FK_A: TestForeignKey = TestForeignKey { target: &A };
        static B_FKS: [&dyn SQLForeignKeyInfo; 1] = [&B_FK_A];
        static B: TestTable = TestTable {
            name: "b",
            fks: &B_FKS,
        };
        static C_FK_B: TestForeignKey = TestForeignKey { target: &B };
        static C_FKS: [&dyn SQLForeignKeyInfo; 1] = [&C_FK_B];
        static C: TestTable = TestTable {
            name: "c",
            fks: &C_FKS,
        };

        let tables: Vec<&dyn SQLTableInfo> = vec![&C, &B, &A];
        let order = seeding_order(&tables);
        assert_eq!(order, vec!["a", "b", "c"]);
    }

    #[test]
    fn no_dependencies() {
        static A: TestTable = TestTable {
            name: "a",
            fks: &[],
        };
        static B: TestTable = TestTable {
            name: "b",
            fks: &[],
        };
        static C: TestTable = TestTable {
            name: "c",
            fks: &[],
        };

        let tables: Vec<&dyn SQLTableInfo> = vec![&C, &B, &A];
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
        static A: TestTable = TestTable {
            name: "a",
            fks: &[],
        };
        static B_FK_A: TestForeignKey = TestForeignKey { target: &A };
        static B_FKS: [&dyn SQLForeignKeyInfo; 1] = [&B_FK_A];
        static B: TestTable = TestTable {
            name: "b",
            fks: &B_FKS,
        };
        static C_FK_A: TestForeignKey = TestForeignKey { target: &A };
        static C_FKS: [&dyn SQLForeignKeyInfo; 1] = [&C_FK_A];
        static C: TestTable = TestTable {
            name: "c",
            fks: &C_FKS,
        };
        static D_FK_B: TestForeignKey = TestForeignKey { target: &B };
        static D_FK_C: TestForeignKey = TestForeignKey { target: &C };
        static D_FKS: [&dyn SQLForeignKeyInfo; 2] = [&D_FK_B, &D_FK_C];
        static D: TestTable = TestTable {
            name: "d",
            fks: &D_FKS,
        };

        let tables: Vec<&dyn SQLTableInfo> = vec![&D, &C, &A, &B];
        let order = seeding_order(&tables);
        // A must come first, then B and C (alphabetical tie-break), then D
        assert_eq!(order, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn cycle_detection() {
        // A -> B -> C -> A (cycle)
        // Cycles should still produce all tables (appended at end)
        static A_FK_C: TestForeignKey = TestForeignKey { target: &C };
        static A_FKS: [&dyn SQLForeignKeyInfo; 1] = [&A_FK_C];
        static A: TestTable = TestTable {
            name: "a",
            fks: &A_FKS,
        };
        static B_FK_A: TestForeignKey = TestForeignKey { target: &A };
        static B_FKS: [&dyn SQLForeignKeyInfo; 1] = [&B_FK_A];
        static B: TestTable = TestTable {
            name: "b",
            fks: &B_FKS,
        };
        static C_FK_B: TestForeignKey = TestForeignKey { target: &B };
        static C_FKS: [&dyn SQLForeignKeyInfo; 1] = [&C_FK_B];
        static C: TestTable = TestTable {
            name: "c",
            fks: &C_FKS,
        };

        let tables: Vec<&dyn SQLTableInfo> = vec![&A, &B, &C];
        let order = seeding_order(&tables);
        // All tables must be present even with cycles
        assert_eq!(order.len(), 3);
        assert!(order.contains(&"a".to_string()));
        assert!(order.contains(&"b".to_string()));
        assert!(order.contains(&"c".to_string()));
    }

    #[test]
    fn self_reference_ignored() {
        // A table that references itself (self-referential FK)
        static A_FK_A: TestForeignKey = TestForeignKey { target: &A };
        static A_FKS: [&dyn SQLForeignKeyInfo; 1] = [&A_FK_A];
        static A: TestTable = TestTable {
            name: "a",
            fks: &A_FKS,
        };

        let tables: Vec<&dyn SQLTableInfo> = vec![&A];
        let order = seeding_order(&tables);
        // Self-references are filtered out (dep_name != table.name())
        assert_eq!(order, vec!["a"]);
    }

    #[test]
    fn single_table() {
        static A: TestTable = TestTable {
            name: "a",
            fks: &[],
        };

        let tables: Vec<&dyn SQLTableInfo> = vec![&A];
        let order = seeding_order(&tables);
        assert_eq!(order, vec!["a"]);
    }

    #[test]
    fn dependency_outside_table_set_ignored() {
        // B depends on X, but X is not in our table set
        static X: TestTable = TestTable {
            name: "x",
            fks: &[],
        };
        static A: TestTable = TestTable {
            name: "a",
            fks: &[],
        };
        static B_FK_X: TestForeignKey = TestForeignKey { target: &X };
        static B_FKS: [&dyn SQLForeignKeyInfo; 1] = [&B_FK_X];
        static B: TestTable = TestTable {
            name: "b",
            fks: &B_FKS,
        };

        // Only pass A and B, not X
        let tables: Vec<&dyn SQLTableInfo> = vec![&B, &A];
        let order = seeding_order(&tables);
        // X is not in the set, so B's dependency on X is ignored
        assert_eq!(order, vec!["a", "b"]);
    }

    #[test]
    fn wide_fan_out() {
        // Parent with many children
        static PARENT: TestTable = TestTable {
            name: "parent",
            fks: &[],
        };
        static C1_FK_PARENT: TestForeignKey = TestForeignKey { target: &PARENT };
        static C1_FKS: [&dyn SQLForeignKeyInfo; 1] = [&C1_FK_PARENT];
        static C1: TestTable = TestTable {
            name: "child_1",
            fks: &C1_FKS,
        };
        static C2_FK_PARENT: TestForeignKey = TestForeignKey { target: &PARENT };
        static C2_FKS: [&dyn SQLForeignKeyInfo; 1] = [&C2_FK_PARENT];
        static C2: TestTable = TestTable {
            name: "child_2",
            fks: &C2_FKS,
        };
        static C3_FK_PARENT: TestForeignKey = TestForeignKey { target: &PARENT };
        static C3_FKS: [&dyn SQLForeignKeyInfo; 1] = [&C3_FK_PARENT];
        static C3: TestTable = TestTable {
            name: "child_3",
            fks: &C3_FKS,
        };
        static C4_FK_PARENT: TestForeignKey = TestForeignKey { target: &PARENT };
        static C4_FKS: [&dyn SQLForeignKeyInfo; 1] = [&C4_FK_PARENT];
        static C4: TestTable = TestTable {
            name: "child_4",
            fks: &C4_FKS,
        };
        static C5_FK_PARENT: TestForeignKey = TestForeignKey { target: &PARENT };
        static C5_FKS: [&dyn SQLForeignKeyInfo; 1] = [&C5_FK_PARENT];
        static C5: TestTable = TestTable {
            name: "child_5",
            fks: &C5_FKS,
        };

        let tables: Vec<&dyn SQLTableInfo> = vec![&C5, &C3, &C1, &PARENT, &C4, &C2];
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
        static A: TestTable = TestTable {
            name: "a",
            fks: &[],
        };
        static B_FK_A: TestForeignKey = TestForeignKey { target: &A };
        static B_FKS: [&dyn SQLForeignKeyInfo; 1] = [&B_FK_A];
        static B: TestTable = TestTable {
            name: "b",
            fks: &B_FKS,
        };
        static C_FK_B: TestForeignKey = TestForeignKey { target: &B };
        static C_FKS: [&dyn SQLForeignKeyInfo; 1] = [&C_FK_B];
        static C: TestTable = TestTable {
            name: "c",
            fks: &C_FKS,
        };
        static D_FK_C: TestForeignKey = TestForeignKey { target: &C };
        static D_FKS: [&dyn SQLForeignKeyInfo; 1] = [&D_FK_C];
        static D: TestTable = TestTable {
            name: "d",
            fks: &D_FKS,
        };
        static E_FK_D: TestForeignKey = TestForeignKey { target: &D };
        static E_FKS: [&dyn SQLForeignKeyInfo; 1] = [&E_FK_D];
        static E: TestTable = TestTable {
            name: "e",
            fks: &E_FKS,
        };

        // Pass in reverse order
        let tables: Vec<&dyn SQLTableInfo> = vec![&E, &D, &C, &B, &A];
        let order = seeding_order(&tables);
        assert_eq!(order, vec!["a", "b", "c", "d", "e"]);
    }
}
