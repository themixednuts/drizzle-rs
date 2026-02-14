#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use crate::common::schema::sqlite::*;
use crate::sqlite::foreign_keys::{CompositeFkSchema, FkCascadeSchema};
use drizzle_seed::{Generator, GeneratorKind, RngCore, SeedConfig, SeedValue};

// ---------------------------------------------------------------------------
// SeedConfig type-safe builder tests
// ---------------------------------------------------------------------------

#[test]
fn config_count_extracts_table_name() {
    let schema = FkCascadeSchema::new();

    let stmts = SeedConfig::sqlite(&schema)
        .count(&schema.fk_parent, 50)
        .count(&schema.fk_cascade, 200)
        .generate();

    // Count rows in parent INSERT
    let parent_sql = stmts
        .iter()
        .map(|s| s.sql())
        .find(|s| s.contains("INSERT INTO") && s.contains("fk_parent"))
        .unwrap();
    let values_section = &parent_sql[parent_sql.find("VALUES ").unwrap() + 7..];
    let parent_rows = values_section.matches('(').count();
    assert_eq!(parent_rows, 50, "fk_parent should have 50 rows");
}

#[test]
fn config_kind_override_via_column_ref() {
    let schema = SimpleSchema::new();

    // Override the "name" column to generate emails instead of names
    let stmts = SeedConfig::sqlite(&schema)
        .seed(42)
        .count(&schema.simple, 5)
        .kind(&Simple::name, GeneratorKind::Email)
        .generate();

    let (sql, params) = stmts[0].build();
    assert!(sql.starts_with("INSERT INTO") && sql.contains("simple"));

    for name_param in params.iter().skip(1).step_by(2) {
        let drizzle::sqlite::values::OwnedSQLiteValue::Text(name_val) = name_param else {
            panic!("expected TEXT for name column");
        };
        assert!(
            name_val.contains('@') && name_val.contains('.'),
            "with Email override, name column should produce emails, got: {name_val}"
        );
    }
}

#[test]
fn config_kind_override_via_schema_config() {
    let schema = SimpleSchema::new();

    let stmts = SeedConfig::sqlite(&schema)
        .seed(42)
        .count(&schema.simple, 5)
        .kind(&Simple::name, GeneratorKind::Email)
        .generate();

    let (_sql, params) = stmts[0].build();
    for name_param in params.iter().skip(1).step_by(2) {
        let drizzle::sqlite::values::OwnedSQLiteValue::Text(name_val) = name_param else {
            panic!("expected TEXT for name column");
        };
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
        fn generate(&self, _rng: &mut dyn RngCore, _index: usize, _sql_type: &str) -> SeedValue {
            SeedValue::Text("FIXED".to_string())
        }
        fn name(&self) -> &'static str {
            "Const"
        }
    }

    let schema = SimpleSchema::new();
    let stmts = SeedConfig::sqlite(&schema)
        .seed(1)
        .count(&schema.simple, 4)
        .generator(&Simple::name, ConstGen)
        .generate();

    let (_sql, params) = stmts[0].build();
    let fixed_count = params
        .iter()
        .skip(1)
        .step_by(2)
        .filter(|v| {
            matches!(
                v,
                drizzle::sqlite::values::OwnedSQLiteValue::Text(s) if s == "FIXED"
            )
        })
        .count();
    assert_eq!(fixed_count, 4, "custom generator should produce FIXED");
}

#[test]
fn config_generator_accepts_column_generator() {
    let schema = SimpleSchema::new();
    let stmts = SeedConfig::sqlite(&schema)
        .seed(1)
        .count(&schema.simple, 4)
        .generator(&Simple::name, &Simple::name)
        .generate();

    let (_sql, params) = stmts[0].build();
    let generated_text_count = params
        .iter()
        .skip(1)
        .step_by(2)
        .filter(|v| matches!(v, drizzle::sqlite::values::OwnedSQLiteValue::Text(_)))
        .count();
    assert_eq!(generated_text_count, 4);
}

#[test]
fn config_skip_parent_allows_explicit_unconstrained_generation() {
    let schema = FkCascadeSchema::new();
    let stmts = SeedConfig::sqlite(&schema)
        .skip(&schema.fk_parent)
        .count(&schema.fk_cascade, 2)
        .generate();

    let sqls: Vec<String> = stmts.iter().map(|s| s.sql()).collect();
    assert!(
        !sqls
            .iter()
            .any(|sql| sql.contains("INSERT INTO") && sql.contains("fk_parent"))
    );
}

#[test]
fn config_skip_child_is_allowed() {
    let schema = FkCascadeSchema::new();
    let stmts = SeedConfig::sqlite(&schema)
        .skip(&schema.fk_cascade)
        .count(&schema.fk_parent, 3)
        .generate();

    let sqls: Vec<String> = stmts.iter().map(|s| s.sql()).collect();
    assert!(
        sqls.iter()
            .any(|sql| sql.contains("INSERT INTO") && sql.contains("fk_parent"))
    );
    assert!(
        !sqls
            .iter()
            .any(|sql| sql.contains("INSERT INTO") && sql.contains("fk_cascade"))
    );
}

#[test]
fn config_relation_via_table_refs() {
    let schema = FkCascadeSchema::new();

    // Just verify it builds without panic
    let _config = SeedConfig::sqlite(&schema)
        .seed(1)
        .count(&schema.fk_parent, 10)
        .count(&schema.fk_cascade, 50)
        .relation(&schema.fk_parent, &schema.fk_cascade, 5);
}

#[test]
fn config_relation_derives_child_count_when_unset() {
    let schema = FkCascadeSchema::new();
    let stmts = SeedConfig::sqlite(&schema)
        .seed(7)
        .count(&schema.fk_parent, 4)
        .relation(&schema.fk_parent, &schema.fk_cascade, 3)
        .generate();

    let child_sql = stmts
        .iter()
        .map(|s| s.sql())
        .find(|s| s.contains("INSERT INTO") && s.contains("fk_cascade"))
        .unwrap();
    let values_start = child_sql.find("VALUES ").unwrap() + 7;
    let values_section = &child_sql[values_start..child_sql.len() - 1];
    let child_rows = values_section.matches('(').count();
    assert_eq!(
        child_rows, 12,
        "relation(3) and parent count 4 should derive 12 child rows"
    );
}

#[test]
fn config_schema_config_includes_all_schema_tables() {
    let schema = FullBlogSchema::new();
    let stmts = SeedConfig::sqlite(&schema)
        .seed(7)
        .count(&schema.simple, 2)
        .count(&schema.complex, 2)
        .count(&schema.post, 2)
        .count(&schema.category, 2)
        .count(&schema.post_category, 2)
        .generate();

    let sqls: Vec<String> = stmts.iter().map(|s| s.sql()).collect();
    for table in [
        "simple",
        "complex",
        "posts",
        "categories",
        "post_categories",
    ] {
        assert!(
            sqls.iter()
                .any(|sql| sql.contains("INSERT INTO") && sql.contains(table)),
            "expected INSERT for table {table}"
        );
    }
}

#[test]
fn fk_child_count_derives_automatically_without_relation() {
    let schema = FkCascadeSchema::new();
    let stmts = SeedConfig::sqlite(&schema)
        .seed(7)
        .count(&schema.fk_parent, 4)
        .generate();

    let child_sql = stmts
        .iter()
        .map(|s| s.sql())
        .find(|s| s.contains("INSERT INTO") && s.contains("fk_cascade"))
        .unwrap();
    let values_start = child_sql.find("VALUES ").unwrap() + 7;
    let values_section = &child_sql[values_start..child_sql.len() - 1];
    let child_rows = values_section.matches('(').count();
    assert_eq!(
        child_rows, 4,
        "without relation, FK child count should default to 1:1 with parent count"
    );
}

#[test]
fn config_relation_groups_fk_values_per_parent() {
    let schema = FkCascadeSchema::new();
    let stmts = SeedConfig::sqlite(&schema)
        .seed(7)
        .count(&schema.fk_parent, 3)
        .relation(&schema.fk_parent, &schema.fk_cascade, 2)
        .generate();

    let (_sql, params) = stmts
        .iter()
        .map(|s| s.build())
        .find(|(sql, _)| sql.contains("INSERT INTO") && sql.contains("fk_cascade"))
        .unwrap();

    let mut parent_ids = Vec::new();
    for parent_param in params.iter().skip(1).step_by(3) {
        let drizzle::sqlite::values::OwnedSQLiteValue::Integer(v) = parent_param else {
            panic!("expected integer parent_id param");
        };
        parent_ids.push(*v);
    }

    assert_eq!(parent_ids, vec![1, 1, 2, 2, 3, 3]);
}

#[test]
fn config_relation_groups_composite_fk_values_per_parent() {
    let schema = CompositeFkSchema::new();
    let stmts = SeedConfig::sqlite(&schema)
        .seed(11)
        .count(&schema.composite_fk_parent, 3)
        .relation(&schema.composite_fk_parent, &schema.composite_fk_child, 2)
        .generate();

    let (_sql, params) = stmts
        .iter()
        .map(|s| s.build())
        .find(|(sql, _)| sql.contains("INSERT INTO") && sql.contains("composite_fk_child"))
        .unwrap();

    let mut fk_pairs = Vec::new();
    for row_params in params.chunks_exact(4) {
        let drizzle::sqlite::values::OwnedSQLiteValue::Integer(parent_a) = row_params[1] else {
            panic!("expected integer parent_a param");
        };
        let drizzle::sqlite::values::OwnedSQLiteValue::Integer(parent_b) = row_params[2] else {
            panic!("expected integer parent_b param");
        };
        fk_pairs.push((parent_a, parent_b));
    }

    assert_eq!(
        fk_pairs,
        vec![(1, 1), (1, 1), (2, 2), (2, 2), (3, 3), (3, 3)]
    );
}

// ---------------------------------------------------------------------------
// Seeder end-to-end: output validation
// ---------------------------------------------------------------------------

#[test]
fn seeder_simple_table_pk_sequential_and_name_inferred() {
    let schema = SimpleSchema::new();
    let stmts = SeedConfig::sqlite(&schema)
        .seed(42)
        .count(&schema.simple, 5)
        .generate();

    assert_eq!(stmts.len(), 1);
    let (sql, params) = stmts[0].build();

    assert!(sql.contains("INSERT INTO") && sql.contains("simple"));
    assert!(sql.contains("VALUES"));

    let mut ids = Vec::new();
    for id_param in params.iter().step_by(2) {
        let drizzle::sqlite::values::OwnedSQLiteValue::Integer(id) = id_param else {
            panic!("expected integer id");
        };
        ids.push(*id);
    }
    assert_eq!(ids, vec![1, 2, 3, 4, 5]);

    for name_param in params.iter().skip(1).step_by(2) {
        let drizzle::sqlite::values::OwnedSQLiteValue::Text(name) = name_param else {
            panic!("expected text name");
        };
        assert!(!name.is_empty());
    }
}

#[test]
fn seeder_deterministic_output() {
    let schema = SimpleSchema::new();
    let config = SeedConfig::sqlite(&schema)
        .seed(123)
        .count(&schema.simple, 20);

    let sql_a: Vec<String> = config.generate().iter().map(|s| s.sql()).collect();
    let sql_b: Vec<String> = config.generate().iter().map(|s| s.sql()).collect();

    assert_eq!(sql_a, sql_b, "same seed must produce identical output");
}

#[test]
fn seeder_different_seeds_produce_different_output() {
    let schema = SimpleSchema::new();

    let stmts_a = SeedConfig::sqlite(&schema)
        .seed(1)
        .count(&schema.simple, 10)
        .generate();
    let stmts_b = SeedConfig::sqlite(&schema)
        .seed(2)
        .count(&schema.simple, 10)
        .generate();

    let params_a = stmts_a[0].build().1;
    let params_b = stmts_b[0].build().1;
    assert_ne!(
        params_a, params_b,
        "different seeds must produce different values"
    );
}

#[test]
fn seeder_fk_parent_before_child() {
    let schema = FkCascadeSchema::new();

    let stmts = SeedConfig::sqlite(&schema)
        .seed(42)
        .count(&schema.fk_parent, 3)
        .count(&schema.fk_cascade, 10)
        .generate();

    let sqls: Vec<String> = stmts.iter().map(|s| s.sql()).collect();

    assert!(sqls.len() >= 2, "should have at least 2 statements");
    assert!(
        sqls[0].contains("INSERT INTO") && sqls[0].contains("fk_parent"),
        "first INSERT should be parent table, got: {}",
        &sqls[0][..60.min(sqls[0].len())]
    );
    assert!(
        sqls[1].contains("INSERT INTO") && sqls[1].contains("fk_cascade"),
        "second INSERT should be child table, got: {}",
        &sqls[1][..60.min(sqls[1].len())]
    );
}

#[test]
fn seeder_fk_values_are_valid_parent_pks() {
    let schema = FkCascadeSchema::new();
    let stmts = SeedConfig::sqlite(&schema)
        .seed(42)
        .count(&schema.fk_parent, 5)
        .count(&schema.fk_cascade, 30)
        .generate();

    // Parent PKs are sequential 1..=5
    let valid_pks: Vec<i64> = (1..=5).collect();

    let (_sql, params) = stmts
        .iter()
        .map(|s| s.build())
        .find(|(sql, _)| sql.contains("INSERT INTO") && sql.contains("fk_cascade"))
        .unwrap();

    for parent_param in params.iter().skip(1).step_by(3) {
        if let drizzle::sqlite::values::OwnedSQLiteValue::Integer(parent_id) = parent_param {
            assert!(
                valid_pks.contains(parent_id),
                "FK parent_id={parent_id} not in valid parent PKs {valid_pks:?}"
            );
        }
    }
}

#[test]
fn seeder_complex_table_column_heuristics() {
    let schema = ComplexSchema::new();
    let stmts = SeedConfig::sqlite(&schema)
        .seed(42)
        .count(&schema.complex, 10)
        .generate();

    assert!(!stmts.is_empty(), "should produce at least one statement");

    let sql = stmts[0].sql();
    assert!(
        sql.contains("INSERT INTO") && sql.contains("complex"),
        "should insert into complex table"
    );

    assert!(sql.contains("id"), "should have id column");
    assert!(sql.contains("email"), "should have email column");
    assert!(sql.contains("name"), "should have name column");
    assert!(
        sql.contains("description"),
        "should have description column"
    );
}

#[test]
fn seeder_zero_count_produces_nothing() {
    let schema = SimpleSchema::new();
    let stmts = SeedConfig::sqlite(&schema)
        .seed(1)
        .count(&schema.simple, 0)
        .generate();

    assert!(stmts.is_empty(), "count=0 should produce no statements");
}

#[test]
fn seeder_default_count_used_when_not_overridden() {
    let schema = SimpleSchema::new();
    // Don't call .count() for simple — should use default_count=7
    let stmts = SeedConfig::sqlite(&schema)
        .seed(1)
        .default_count(7)
        .generate();

    let sql = stmts[0].sql();
    let value_section = &sql[sql.find("VALUES ").unwrap() + 7..];
    let row_count = value_section.matches('(').count();
    assert_eq!(
        row_count, 7,
        "default_count=7 should produce 7 rows, got {row_count}"
    );
}

#[test]
fn seeder_multi_table_blog_schema() {
    let schema = FullBlogSchema::new();
    let stmts = SeedConfig::sqlite(&schema)
        .seed(42)
        .count(&schema.simple, 3)
        .count(&schema.complex, 5)
        .count(&schema.post, 10)
        .count(&schema.category, 4)
        .count(&schema.post_category, 15)
        .generate();

    let sqls: Vec<String> = stmts.iter().map(|s| s.sql()).collect();

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
            sqls.iter()
                .any(|s| s.contains("INSERT INTO") && s.contains(name)),
            "should have INSERT for table {name}"
        );
    }

    // Verify ordering: complex must come before posts (posts has FK to complex)
    let complex_idx = sqls
        .iter()
        .position(|s| s.contains("INSERT INTO") && s.contains("complex"))
        .unwrap();
    let posts_idx = sqls
        .iter()
        .position(|s| s.contains("INSERT INTO") && s.contains("posts"))
        .unwrap();
    assert!(
        complex_idx < posts_idx,
        "complex (parent) must be seeded before posts (child)"
    );
}

#[test]
fn seeder_statements_return_sql_and_params() {
    let schema = SimpleSchema::new();
    let stmts = SeedConfig::sqlite(&schema)
        .seed(42)
        .count(&schema.simple, 3)
        .generate();

    assert_eq!(stmts.len(), 1);

    let (sql, params) = stmts[0].build();
    assert!(sql.starts_with("INSERT INTO"));
    assert!(sql.contains('?'));
    assert_eq!(params.len(), 6);
}

#[test]
fn seeder_respects_max_params() {
    let schema = SimpleSchema::new();
    let stmts = SeedConfig::sqlite(&schema)
        .seed(42)
        .count(&schema.simple, 10)
        .max_params(4)
        .generate();

    // simple has 2 params/row => limit 4 means 2 rows per statement => 5 statements
    assert_eq!(stmts.len(), 5);
    for stmt in &stmts {
        let (_sql, params) = stmt.build();
        assert!(params.len() <= 4);
    }
}

#[test]
fn seeder_email_column_produces_emails() {
    let schema = ComplexSchema::new();
    let stmts = SeedConfig::sqlite(&schema)
        .seed(42)
        .count(&schema.complex, 10)
        .generate();

    let (_sql, params) = stmts[0].build();
    let mut email_count = 0;
    for param in &params {
        if let drizzle::sqlite::values::OwnedSQLiteValue::Text(email_text) = param
            && email_text.contains('@')
            && email_text.contains('.')
        {
            email_count += 1;
        }
    }
    assert!(
        email_count > 0,
        "should have produced at least one non-NULL email"
    );
}
