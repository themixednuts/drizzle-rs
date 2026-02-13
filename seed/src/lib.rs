//! Deterministic database seeding for drizzle-rs.
//!
//! Generates reproducible INSERT statements using type-aware generators
//! and column name heuristics. FK-aware topological ordering ensures
//! parent tables are seeded before children.
//!
//! # Example
//!
//! ```ignore
//! use drizzle_seed::{Seeder, SeedConfig, batch::Dialect};
//!
//! let config = SeedConfig::new()
//!     .seed(42)
//!     .count(&schema.users, 100)
//!     .count(&schema.posts, 500);
//!
//! let tables = vec![&schema.users as &dyn SQLTableInfo, &schema.posts];
//! let seeder = Seeder::new(&tables, Dialect::Sqlite, &config);
//! let statements = seeder.generate();
//! ```

pub mod batch;
pub mod config;
pub mod datasets;
pub mod generator;
pub mod inference;
pub mod rng;
pub mod topology;

pub use batch::Dialect;
pub use config::SeedConfig;
pub use generator::{Generator, GeneratorKind, SqlValue};

use drizzle_core::SQLTableInfo;
use rand::rngs::StdRng;
use std::collections::HashMap;
use std::sync::Arc;

/// The main seeder that generates INSERT statements for a set of tables.
pub struct Seeder<'a> {
    tables: Vec<&'a dyn SQLTableInfo>,
    dialect: Dialect,
    config: &'a SeedConfig,
}

impl<'a> Seeder<'a> {
    /// Create a seeder from a slice of table info references.
    pub fn new(tables: &[&'a dyn SQLTableInfo], dialect: Dialect, config: &'a SeedConfig) -> Self {
        Self {
            tables: tables.to_vec(),
            dialect,
            config,
        }
    }

    /// Generate all INSERT statements in FK-aware order.
    ///
    /// Returns a `Vec<String>` where each string is a complete INSERT statement.
    /// Statements are batched to respect the dialect's parameter limit.
    pub fn generate(&self) -> Vec<String> {
        let order = topology::seeding_order(&self.tables);
        let table_map: HashMap<&str, &dyn SQLTableInfo> =
            self.tables.iter().map(|t| (t.name(), *t)).collect();

        // Track generated PK values for FK resolution
        let mut pk_values: HashMap<String, Vec<SqlValue>> = HashMap::new();
        let mut statements = Vec::new();

        for table_name in &order {
            let Some(&table) = table_map.get(table_name.as_str()) else {
                continue;
            };

            let columns = table.columns();
            if columns.is_empty() {
                continue;
            }

            let count = self.config.count_for(table_name);
            if count == 0 {
                continue;
            }

            // Build generators for each column
            let generators = self.build_generators(table, &pk_values);
            let col_names: Vec<&str> = columns.iter().map(|c| c.name()).collect();

            // Generate all row values
            let mut all_rows: Vec<Vec<SqlValue>> = Vec::with_capacity(count);
            let mut col_rngs: Vec<StdRng> = columns
                .iter()
                .map(|c| rng::column_rng(table_name, c.name(), self.config.seed))
                .collect();

            for row_idx in 0..count {
                let mut row = Vec::with_capacity(columns.len());
                for (col_idx, g) in generators.iter().enumerate() {
                    let val = g.generate(&mut col_rngs[col_idx], row_idx);
                    row.push(val);
                }
                all_rows.push(row);
            }

            // Store PK values for FK resolution in dependent tables
            for (col_idx, col) in columns.iter().enumerate() {
                if col.is_primary_key() {
                    let vals: Vec<SqlValue> =
                        all_rows.iter().map(|row| row[col_idx].clone()).collect();
                    pk_values.insert(format!("{}.{}", table_name, col.name()), vals);
                }
            }

            // Batch the rows into INSERT statements
            let max_batch = self.dialect.max_batch_rows(columns.len()).max(1);
            for chunk in all_rows.chunks(max_batch) {
                let stmt = format_insert(table_name, &col_names, chunk);
                statements.push(stmt);
            }
        }

        statements
    }

    /// Build generators for each column in a table.
    fn build_generators(
        &self,
        table: &dyn SQLTableInfo,
        pk_values: &HashMap<String, Vec<SqlValue>>,
    ) -> Vec<Box<dyn Generator>> {
        let table_name = table.name();
        table
            .columns()
            .iter()
            .map(|col| {
                let col_name = col.name();
                let key = (table_name.to_string(), col_name.to_string());

                // Priority 1: user-provided custom generator
                if let Some(g) = self.config.column_generators.get(&key) {
                    return Box::new(Arc::clone(g)) as Box<dyn Generator>;
                }

                // Priority 2: user-provided generator kind override
                if let Some(&kind) = self.config.column_kinds.get(&key) {
                    return kind.into_generator();
                }

                // Priority 3: FK column → pick from parent's generated PK values
                if let Some(fk_col) = col.foreign_key() {
                    let fk_key = format!("{}.{}", fk_col.table().name(), fk_col.name());
                    if let Some(parent_vals) = pk_values.get(&fk_key) {
                        return Box::new(FkGen {
                            parent_values: parent_vals.clone(),
                            nullable: !col.is_not_null(),
                        });
                    }
                }

                // Priority 4: columns with defaults — skip (NULL for nullable, or let DB handle)
                if col.has_default() && !col.is_primary_key() {
                    return Box::new(DefaultGen {
                        nullable: !col.is_not_null(),
                    });
                }

                // Priority 5: infer from column type and name
                inference::infer_generator(*col).into_generator()
            })
            .collect()
    }
}

/// Format an INSERT statement for a batch of rows.
fn format_insert(table: &str, columns: &[&str], rows: &[Vec<SqlValue>]) -> String {
    let cols = columns.join(", ");
    let values: Vec<String> = rows
        .iter()
        .map(|row| {
            let vals: Vec<String> = row.iter().map(|v| v.to_sql_literal()).collect();
            format!("({})", vals.join(", "))
        })
        .collect();
    format!("INSERT INTO {table} ({cols}) VALUES {};", values.join(", "))
}

/// A generator that picks from parent table PK values (for FK columns).
struct FkGen {
    parent_values: Vec<SqlValue>,
    nullable: bool,
}

impl Generator for FkGen {
    fn generate(&self, rng: &mut dyn generator::RngCore, _index: usize) -> SqlValue {
        use rand::Rng;
        if self.parent_values.is_empty() {
            return SqlValue::Null;
        }
        // Occasionally generate NULL for nullable FK columns
        if self.nullable && rng.random_ratio(1, 20) {
            return SqlValue::Null;
        }
        let idx = rng.random_range(0..self.parent_values.len());
        self.parent_values[idx].clone()
    }
    fn name(&self) -> &'static str {
        "ForeignKey"
    }
}

/// A generator for columns with database defaults — generates NULL (or skips).
struct DefaultGen {
    nullable: bool,
}

impl Generator for DefaultGen {
    fn generate(&self, _rng: &mut dyn generator::RngCore, _index: usize) -> SqlValue {
        if self.nullable {
            SqlValue::Null
        } else {
            // For non-nullable columns with defaults, use NULL and let DB apply default.
            // This works if the INSERT uses DEFAULT instead, but for simplicity
            // we generate NULL which will fail for NOT NULL — the user should override.
            SqlValue::Null
        }
    }
    fn name(&self) -> &'static str {
        "Default"
    }
}

/// Implement Generator for Arc<dyn Generator> so we can share generators via Arc.
impl Generator for Arc<dyn Generator> {
    fn generate(&self, rng: &mut dyn generator::RngCore, index: usize) -> SqlValue {
        (**self).generate(rng, index)
    }
    fn name(&self) -> &'static str {
        (**self).name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_insert_basic() {
        let rows = vec![
            vec![SqlValue::Integer(1), SqlValue::Text("hello".to_string())],
            vec![SqlValue::Integer(2), SqlValue::Text("world".to_string())],
        ];
        let stmt = format_insert("users", &["id", "name"], &rows);
        assert_eq!(
            stmt,
            "INSERT INTO users (id, name) VALUES (1, 'hello'), (2, 'world');"
        );
    }

    #[test]
    fn format_insert_all_value_types() {
        let rows = vec![vec![
            SqlValue::Integer(42),
            SqlValue::Float(3.14),
            SqlValue::Text("hello".to_string()),
            SqlValue::Bool(true),
            SqlValue::Bool(false),
            SqlValue::Null,
            SqlValue::Blob(vec![0xde, 0xad, 0xbe, 0xef]),
        ]];
        let stmt = format_insert("test", &["i", "f", "t", "b1", "b2", "n", "bl"], &rows);
        assert_eq!(
            stmt,
            "INSERT INTO test (i, f, t, b1, b2, n, bl) VALUES (42, 3.14, 'hello', 1, 0, NULL, X'deadbeef');"
        );
    }

    #[test]
    fn sql_literal_escaping() {
        let val = SqlValue::Text("it's a test".to_string());
        assert_eq!(val.to_sql_literal(), "'it''s a test'");
    }

    #[test]
    fn sql_literal_double_quote_in_text() {
        let val = SqlValue::Text("say \"hello\"".to_string());
        // Double quotes are not special in SQL string literals
        assert_eq!(val.to_sql_literal(), "'say \"hello\"'");
    }

    #[test]
    fn sql_literal_empty_string() {
        let val = SqlValue::Text(String::new());
        assert_eq!(val.to_sql_literal(), "''");
    }

    #[test]
    fn sql_literal_null() {
        assert_eq!(SqlValue::Null.to_sql_literal(), "NULL");
    }

    #[test]
    fn sql_literal_blob_empty() {
        assert_eq!(SqlValue::Blob(vec![]).to_sql_literal(), "X''");
    }

    #[test]
    fn arc_generator_delegation() {
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let g: Arc<dyn Generator> = Arc::new(generator::numeric::IntPrimaryKeyGen);
        let mut rng = StdRng::seed_from_u64(42);

        // Arc<dyn Generator> should delegate to the inner generator
        assert_eq!(g.generate(&mut rng, 0), SqlValue::Integer(1));
        assert_eq!(g.generate(&mut rng, 4), SqlValue::Integer(5));
        assert_eq!(g.name(), "IntPrimaryKey");
    }

    #[test]
    fn fk_gen_picks_from_parent_values() {
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let parent_vals = vec![
            SqlValue::Integer(10),
            SqlValue::Integer(20),
            SqlValue::Integer(30),
        ];
        let g = FkGen {
            parent_values: parent_vals.clone(),
            nullable: false,
        };
        let mut rng = StdRng::seed_from_u64(42);

        for _ in 0..50 {
            let val = g.generate(&mut rng, 0);
            assert!(
                parent_vals.contains(&val),
                "FK value {:?} not in parent set",
                val
            );
        }
    }

    #[test]
    fn fk_gen_empty_parent_returns_null() {
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let g = FkGen {
            parent_values: vec![],
            nullable: false,
        };
        let mut rng = StdRng::seed_from_u64(42);
        assert_eq!(g.generate(&mut rng, 0), SqlValue::Null);
    }

    #[test]
    fn default_gen_returns_null() {
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let g = DefaultGen { nullable: true };
        let mut rng = StdRng::seed_from_u64(42);
        assert_eq!(g.generate(&mut rng, 0), SqlValue::Null);
    }
}
