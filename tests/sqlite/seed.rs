#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use crate::common::schema::sqlite::*;
use crate::sqlite::foreign_keys::{FkCascade, FkCascadeSchema, FkParent};
use drizzle::core::SQLTableInfo;
use drizzle_seed::{Dialect, Generator, GeneratorKind, SeedConfig, Seeder, SqlValue};

// ---------------------------------------------------------------------------
// SeedConfig type-safe builder tests
// ---------------------------------------------------------------------------

#[test]
fn config_count_extracts_table_name() {
    let schema = FkCascadeSchema::new();

    let config = SeedConfig::new()
        .count(&schema.fk_parent, 50)
        .count(&schema.fk_cascade, 200);

    // count_for is pub(crate), so verify indirectly via Seeder output
    let tables: Vec<&dyn SQLTableInfo> = vec![&schema.fk_parent, &schema.fk_cascade];
    let stmts = Seeder::new(&tables, Dialect::Sqlite, &config).generate();

    // Count rows in parent INSERT
    let parent_stmt = stmts
        .iter()
        .find(|s| s.starts_with("INSERT INTO fk_parent"))
        .unwrap();
    let values_section = &parent_stmt[parent_stmt.find("VALUES ").unwrap() + 7..];
    let parent_rows = values_section.matches('(').count();
    assert_eq!(parent_rows, 50, "fk_parent should have 50 rows");
}

#[test]
fn config_kind_override_via_column_ref() {
    let schema = SimpleSchema::new();

    // Override the "name" column to generate emails instead of names
    let config = SeedConfig::new()
        .seed(42)
        .count(&schema.simple, 5)
        .kind(&Simple::name, GeneratorKind::Email);

    let tables: Vec<&dyn SQLTableInfo> = vec![&schema.simple];
    let stmts = Seeder::new(&tables, Dialect::Sqlite, &config).generate();

    let stmt = &stmts[0];
    assert!(stmt.starts_with("INSERT INTO simple"));

    // Parse out name values (second column in each tuple)
    let values_start = stmt.find("VALUES ").unwrap() + 7;
    let values_section = &stmt[values_start..stmt.len() - 1];
    for tuple_str in values_section.split("), (") {
        let clean = tuple_str.trim_start_matches('(').trim_end_matches(')');
        let fields: Vec<&str> = clean.splitn(2, ", ").collect();
        let name_val = fields[1].trim().trim_matches('\'');
        assert!(
            name_val.contains('@') && name_val.contains('.'),
            "with Email override, name column should produce emails, got: {name_val}"
        );
    }
}

#[test]
fn config_custom_generator_via_column_ref() {
    struct ConstGen;
    impl Generator for ConstGen {
        fn generate(
            &self,
            _rng: &mut dyn drizzle_seed::generator::RngCore,
            _index: usize,
        ) -> SqlValue {
            SqlValue::Text("FIXED".to_string())
        }
        fn name(&self) -> &'static str {
            "Const"
        }
    }

    let schema = SimpleSchema::new();
    let config = SeedConfig::new()
        .seed(1)
        .count(&schema.simple, 4)
        .generator(&Simple::name, Box::new(ConstGen));

    let tables: Vec<&dyn SQLTableInfo> = vec![&schema.simple];
    let stmts = Seeder::new(&tables, Dialect::Sqlite, &config).generate();

    let stmt = &stmts[0];
    let fixed_count = stmt.matches("'FIXED'").count();
    assert_eq!(
        fixed_count, 4,
        "custom generator should produce 'FIXED' for all 4 rows, got {fixed_count}"
    );
}

#[test]
fn config_with_relation_via_table_refs() {
    let schema = FkCascadeSchema::new();

    // Just verify it builds without panic — relation_counts is pub(crate)
    // so we test indirectly via the fact that the config is accepted
    let _config = SeedConfig::new()
        .seed(1)
        .count(&schema.fk_parent, 10)
        .count(&schema.fk_cascade, 50)
        .with_relation(&schema.fk_parent, &schema.fk_cascade, 5);
}

// ---------------------------------------------------------------------------
// Seeder end-to-end: output validation
// ---------------------------------------------------------------------------

#[test]
fn seeder_simple_table_pk_sequential_and_name_inferred() {
    let schema = SimpleSchema::new();
    let config = SeedConfig::new().seed(42).count(&schema.simple, 5);

    let tables: Vec<&dyn SQLTableInfo> = vec![&schema.simple];
    let stmts = Seeder::new(&tables, Dialect::Sqlite, &config).generate();

    assert_eq!(stmts.len(), 1);
    let stmt = &stmts[0];

    // Verify INSERT structure
    assert!(stmt.starts_with("INSERT INTO simple (id, name) VALUES "));
    assert!(stmt.ends_with(';'));

    // PKs should be sequential 1..=5
    for i in 1..=5 {
        assert!(
            stmt.contains(&format!("({i}, ")),
            "should contain PK={i}: {stmt}"
        );
    }

    // "name" column → FullName heuristic → should produce non-empty quoted strings
    let values_start = stmt.find("VALUES ").unwrap() + 7;
    let values_section = &stmt[values_start..stmt.len() - 1];
    for tuple_str in values_section.split("), (") {
        let clean = tuple_str.trim_start_matches('(').trim_end_matches(')');
        let fields: Vec<&str> = clean.splitn(2, ", ").collect();
        let name_val = fields[1].trim();
        assert!(
            name_val.starts_with('\'') && name_val.ends_with('\''),
            "name should be a quoted string, got: {name_val}"
        );
        let inner = name_val.trim_matches('\'');
        assert!(!inner.is_empty(), "name should not be empty");
    }
}

#[test]
fn seeder_deterministic_output() {
    let schema = SimpleSchema::new();
    let config = SeedConfig::new().seed(123).count(&schema.simple, 20);

    let tables: Vec<&dyn SQLTableInfo> = vec![&schema.simple];
    let stmts_a = Seeder::new(&tables, Dialect::Sqlite, &config).generate();
    let stmts_b = Seeder::new(&tables, Dialect::Sqlite, &config).generate();

    assert_eq!(stmts_a, stmts_b, "same seed must produce identical output");
}

#[test]
fn seeder_different_seeds_produce_different_output() {
    let schema = SimpleSchema::new();
    let tables: Vec<&dyn SQLTableInfo> = vec![&schema.simple];

    let config_a = SeedConfig::new().seed(1).count(&schema.simple, 10);
    let config_b = SeedConfig::new().seed(2).count(&schema.simple, 10);

    let stmts_a = Seeder::new(&tables, Dialect::Sqlite, &config_a).generate();
    let stmts_b = Seeder::new(&tables, Dialect::Sqlite, &config_b).generate();

    assert_ne!(
        stmts_a, stmts_b,
        "different seeds must produce different output"
    );
}

#[test]
fn seeder_fk_parent_before_child() {
    let schema = FkCascadeSchema::new();
    let config = SeedConfig::new()
        .seed(42)
        .count(&schema.fk_parent, 3)
        .count(&schema.fk_cascade, 10);

    // Pass child first — seeder should still emit parent first
    let tables: Vec<&dyn SQLTableInfo> = vec![&schema.fk_cascade, &schema.fk_parent];
    let stmts = Seeder::new(&tables, Dialect::Sqlite, &config).generate();

    assert!(stmts.len() >= 2, "should have at least 2 statements");
    assert!(
        stmts[0].starts_with("INSERT INTO fk_parent"),
        "first INSERT should be parent table, got: {}",
        &stmts[0][..60.min(stmts[0].len())]
    );
    assert!(
        stmts[1].starts_with("INSERT INTO fk_cascade"),
        "second INSERT should be child table, got: {}",
        &stmts[1][..60.min(stmts[1].len())]
    );
}

#[test]
fn seeder_fk_values_are_valid_parent_pks() {
    let schema = FkCascadeSchema::new();
    let config = SeedConfig::new()
        .seed(42)
        .count(&schema.fk_parent, 5)
        .count(&schema.fk_cascade, 30);

    let tables: Vec<&dyn SQLTableInfo> = vec![&schema.fk_parent, &schema.fk_cascade];
    let stmts = Seeder::new(&tables, Dialect::Sqlite, &config).generate();

    // Parent PKs are sequential 1..=5
    let valid_pks: Vec<i64> = (1..=5).collect();

    // Parse child statement — columns are (id, parent_id, value)
    let child_stmt = stmts
        .iter()
        .find(|s| s.starts_with("INSERT INTO fk_cascade"))
        .unwrap();
    let values_start = child_stmt.find("VALUES ").unwrap() + 7;
    let values_section = &child_stmt[values_start..child_stmt.len() - 1];

    for tuple_str in values_section.split("), (") {
        let clean = tuple_str.trim_start_matches('(').trim_end_matches(')');
        let fields: Vec<&str> = clean.splitn(3, ", ").collect();
        let parent_id_str = fields[1].trim();

        // parent_id is Option<i32>, so it could be NULL or an integer
        if parent_id_str != "NULL" {
            let parent_id: i64 = parent_id_str
                .parse()
                .unwrap_or_else(|_| panic!("parent_id should be integer, got: {parent_id_str}"));
            assert!(
                valid_pks.contains(&parent_id),
                "FK parent_id={parent_id} not in valid parent PKs {valid_pks:?}"
            );
        }
    }
}

#[test]
fn seeder_complex_table_column_heuristics() {
    let schema = ComplexSchema::new();
    let config = SeedConfig::new().seed(42).count(&schema.complex, 10);

    let tables: Vec<&dyn SQLTableInfo> = vec![&schema.complex];
    let stmts = Seeder::new(&tables, Dialect::Sqlite, &config).generate();

    assert!(!stmts.is_empty(), "should produce at least one statement");

    let stmt = &stmts[0];
    assert!(
        stmt.starts_with("INSERT INTO complex"),
        "should insert into complex table"
    );

    // The Complex table has columns: id, name, email, age, score, active, role, description, data_blob, created_at
    // Verify column names appear in the INSERT
    assert!(
        stmt.contains("id,") || stmt.contains("id)"),
        "should have id column"
    );
    assert!(stmt.contains("email"), "should have email column");
    assert!(stmt.contains("name"), "should have name column");
    assert!(
        stmt.contains("description"),
        "should have description column"
    );
}

#[test]
fn seeder_zero_count_produces_nothing() {
    let schema = SimpleSchema::new();
    let config = SeedConfig::new().seed(1).count(&schema.simple, 0);

    let tables: Vec<&dyn SQLTableInfo> = vec![&schema.simple];
    let stmts = Seeder::new(&tables, Dialect::Sqlite, &config).generate();

    assert!(stmts.is_empty(), "count=0 should produce no statements");
}

#[test]
fn seeder_default_count_used_when_not_overridden() {
    let schema = SimpleSchema::new();
    let config = SeedConfig::new().seed(1).default_count(7);
    // Don't call .count() for simple — should use default_count=7

    let tables: Vec<&dyn SQLTableInfo> = vec![&schema.simple];
    let stmts = Seeder::new(&tables, Dialect::Sqlite, &config).generate();

    let stmt = &stmts[0];
    let value_section = &stmt[stmt.find("VALUES ").unwrap() + 7..];
    let row_count = value_section.matches('(').count();
    assert_eq!(
        row_count, 7,
        "default_count=7 should produce 7 rows, got {row_count}"
    );
}

#[test]
fn seeder_multi_table_blog_schema() {
    // Use FullBlogSchema which has simple, complex, post, category, post_category
    let schema = FullBlogSchema::new();
    let config = SeedConfig::new()
        .seed(42)
        .count(&schema.simple, 3)
        .count(&schema.complex, 5)
        .count(&schema.post, 10)
        .count(&schema.category, 4)
        .count(&schema.post_category, 15);

    let tables: Vec<&dyn SQLTableInfo> = vec![
        &schema.simple,
        &schema.complex,
        &schema.post,
        &schema.category,
        &schema.post_category,
    ];
    let stmts = Seeder::new(&tables, Dialect::Sqlite, &config).generate();

    // Verify all tables got INSERT statements
    let table_names = [
        "simple",
        "complex",
        "posts",
        "categories",
        "post_categories",
    ];
    for name in &table_names {
        assert!(
            stmts
                .iter()
                .any(|s| s.starts_with(&format!("INSERT INTO {name}"))),
            "should have INSERT for table {name}"
        );
    }

    // Verify ordering: complex must come before posts (posts has FK to complex)
    let complex_idx = stmts
        .iter()
        .position(|s| s.starts_with("INSERT INTO complex"))
        .unwrap();
    let posts_idx = stmts
        .iter()
        .position(|s| s.starts_with("INSERT INTO posts"))
        .unwrap();
    assert!(
        complex_idx < posts_idx,
        "complex (parent) must be seeded before posts (child)"
    );
}

#[test]
fn seeder_postgres_dialect_generates_valid_sql() {
    let schema = SimpleSchema::new();
    let config = SeedConfig::new().seed(42).count(&schema.simple, 3);

    let tables: Vec<&dyn SQLTableInfo> = vec![&schema.simple];
    let stmts = Seeder::new(&tables, Dialect::Postgres, &config).generate();

    // Postgres dialect should produce the same SQL structure
    assert_eq!(stmts.len(), 1);
    let stmt = &stmts[0];
    assert!(stmt.starts_with("INSERT INTO simple (id, name) VALUES "));
    assert!(stmt.ends_with(';'));

    // 3 rows
    let value_section = &stmt[stmt.find("VALUES ").unwrap() + 7..];
    let row_count = value_section.matches('(').count();
    assert_eq!(row_count, 3);
}

#[test]
fn seeder_email_column_produces_emails() {
    let schema = ComplexSchema::new();
    let config = SeedConfig::new().seed(42).count(&schema.complex, 10);

    let tables: Vec<&dyn SQLTableInfo> = vec![&schema.complex];
    let stmts = Seeder::new(&tables, Dialect::Sqlite, &config).generate();

    let stmt = &stmts[0];
    // The Complex table has an "email" column — verify generated values contain '@'
    // Find the position of "email" in the column list to know which field index to check
    let col_section = &stmt[stmt.find('(').unwrap() + 1..stmt.find(')').unwrap()];
    let columns: Vec<&str> = col_section.split(", ").collect();
    let email_idx = columns
        .iter()
        .position(|c| *c == "email")
        .expect("Complex table should have an email column");

    let values_start = stmt.find("VALUES ").unwrap() + 7;
    let values_section = &stmt[values_start..stmt.len() - 1];

    let mut email_count = 0;
    for tuple_str in values_section.split("), (") {
        let clean = tuple_str.trim_start_matches('(').trim_end_matches(')');
        // Split into the right number of fields
        let fields: Vec<&str> = clean.splitn(columns.len(), ", ").collect();
        if fields.len() > email_idx {
            let email_val = fields[email_idx].trim();
            // email is Option<String> so it could be NULL
            if email_val != "NULL" {
                let email_text = email_val.trim_matches('\'');
                assert!(
                    email_text.contains('@') && email_text.contains('.'),
                    "email column should produce valid emails, got: {email_text}"
                );
                email_count += 1;
            }
        }
    }
    assert!(
        email_count > 0,
        "should have produced at least one non-NULL email"
    );
}
