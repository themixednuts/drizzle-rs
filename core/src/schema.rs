use crate::prelude::*;
use crate::{ToSQL, sql::SQL, traits::SQLParam};
use core::any::Any;

#[cfg(feature = "std")]
use std::collections::BTreeSet;

/// Trait for database enum types that can be part of a schema
pub trait SQLEnumInfo: Any + Send + Sync {
    /// The name of this enum type
    fn name(&self) -> &'static str;

    /// The SQL CREATE TYPE statement for this enum
    fn create_type_sql(&self) -> String;

    /// All possible values of this enum
    fn variants(&self) -> &'static [&'static str];
}

impl core::fmt::Debug for dyn SQLEnumInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SQLEnumInfo")
            .field("name", &self.name())
            .field("variants", &self.variants())
            .finish()
    }
}

/// Sort direction for ORDER BY clauses
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderBy {
    Asc,
    Desc,
}

/// Creates an ascending ORDER BY expression: "column ASC"
pub fn asc<'a, V, T>(column: T) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    column.to_sql().append(&OrderBy::Asc)
}

/// Creates a descending ORDER BY expression: "column DESC"
pub fn desc<'a, V, T>(column: T) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    column.to_sql().append(&OrderBy::Desc)
}

/// Topological sort of `(name, dependency_names)` pairs using Kahn's algorithm.
///
/// Returns names in dependency order (dependencies before dependents).
/// Uses `BTreeSet` for deterministic tie-breaking (lexicographic).
///
/// # Errors
///
/// Returns an error if a cycle is detected.
#[cfg(feature = "std")]
#[allow(dead_code)]
pub(crate) fn topological_order<'a>(
    items: impl IntoIterator<Item = (&'a str, &'a [&'a str])>,
) -> crate::error::Result<Vec<&'a str>> {
    let items: Vec<_> = items.into_iter().collect();
    let name_set: HashMap<&str, usize> = items
        .iter()
        .enumerate()
        .map(|(i, (name, _))| (*name, i))
        .collect();

    let n = items.len();
    let mut indegree = vec![0usize; n];
    let mut reverse_edges: Vec<Vec<usize>> = vec![Vec::new(); n];

    for (i, (_name, deps)) in items.iter().enumerate() {
        for dep in *deps {
            if let Some(&j) = name_set.get(dep) {
                indegree[i] += 1;
                reverse_edges[j].push(i);
            }
        }
    }

    let mut queue: BTreeSet<(&str, usize)> = BTreeSet::new();
    for (i, &deg) in indegree.iter().enumerate() {
        if deg == 0 {
            queue.insert((items[i].0, i));
        }
    }

    let mut result = Vec::with_capacity(n);
    while let Some(&entry) = queue.first() {
        queue.remove(&entry);
        let (_name, idx) = entry;
        result.push(items[idx].0);

        for &neighbor in &reverse_edges[idx] {
            indegree[neighbor] -= 1;
            if indegree[neighbor] == 0 {
                queue.insert((items[neighbor].0, neighbor));
            }
        }
    }

    if result.len() != n {
        return Err(crate::error::DrizzleError::Schema(
            "cycle detected in table dependencies".into(),
        ));
    }

    Ok(result)
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for OrderBy {
    fn to_sql(&self) -> SQL<'a, V> {
        let sql_str = match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        };
        SQL::raw(sql_str)
    }
}
