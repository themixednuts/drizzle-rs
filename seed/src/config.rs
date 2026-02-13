//! Seeder configuration with builder pattern.

use crate::generator::{Generator, GeneratorKind};
use drizzle_core::{SQLColumnInfo, SQLTableInfo};
use std::collections::HashMap;
use std::sync::Arc;

/// Configuration for seeding a database.
pub struct SeedConfig {
    /// User-provided seed for deterministic RNG.
    pub(crate) seed: u64,
    /// Default number of rows per table if not overridden.
    pub(crate) default_count: usize,
    /// Per-table row count overrides.
    pub(crate) table_counts: HashMap<String, usize>,
    /// Per-column generator overrides. Key: (table_name, column_name).
    pub(crate) column_generators: HashMap<(String, String), Arc<dyn Generator>>,
    /// Per-column generator kind overrides (simpler than full Generator).
    pub(crate) column_kinds: HashMap<(String, String), GeneratorKind>,
    /// Relation cardinality overrides. Key: (parent_table, child_table) -> children per parent.
    pub(crate) relation_counts: HashMap<(String, String), usize>,
}

impl Default for SeedConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            default_count: 10,
            table_counts: HashMap::new(),
            column_generators: HashMap::new(),
            column_kinds: HashMap::new(),
            relation_counts: HashMap::new(),
        }
    }
}

impl SeedConfig {
    /// Create a new config with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the random seed for deterministic generation.
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Set the default row count for all tables.
    pub fn default_count(mut self, count: usize) -> Self {
        self.default_count = count;
        self
    }

    /// Set the row count for a specific table.
    ///
    /// Accepts any `&dyn SQLTableInfo` — use your schema table directly:
    /// ```ignore
    /// config.count(&schema.users, 100)
    /// ```
    pub fn count(mut self, table: &dyn SQLTableInfo, count: usize) -> Self {
        self.table_counts.insert(table.name().to_string(), count);
        self
    }

    /// Override the generator for a specific column using a GeneratorKind.
    ///
    /// The column knows its own table, so you only pass the column:
    /// ```ignore
    /// config.kind(&Users::email, GeneratorKind::Email)
    /// ```
    pub fn kind(mut self, column: &dyn SQLColumnInfo, kind: GeneratorKind) -> Self {
        let table_name = column.table().name().to_string();
        let col_name = column.name().to_string();
        self.column_kinds.insert((table_name, col_name), kind);
        self
    }

    /// Override the generator for a specific column with a custom Generator.
    ///
    /// The column knows its own table, so you only pass the column:
    /// ```ignore
    /// config.generator(&Users::age, Box::new(my_custom_gen))
    /// ```
    pub fn generator(mut self, column: &dyn SQLColumnInfo, g: Box<dyn Generator>) -> Self {
        let table_name = column.table().name().to_string();
        let col_name = column.name().to_string();
        self.column_generators
            .insert((table_name, col_name), Arc::from(g));
        self
    }

    /// Set how many child rows to generate per parent row for a relation.
    ///
    /// ```ignore
    /// config.with_relation(&schema.users, &schema.posts, 5)
    /// ```
    pub fn with_relation(
        mut self,
        parent: &dyn SQLTableInfo,
        child: &dyn SQLTableInfo,
        count: usize,
    ) -> Self {
        self.relation_counts
            .insert((parent.name().to_string(), child.name().to_string()), count);
        self
    }

    /// Get the row count for a table.
    pub(crate) fn count_for(&self, table: &str) -> usize {
        self.table_counts
            .get(table)
            .copied()
            .unwrap_or(self.default_count)
    }
}
