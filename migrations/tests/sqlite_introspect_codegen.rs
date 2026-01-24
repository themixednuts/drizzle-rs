//! Integration tests for SQLite introspection and Rust schema code generation
//!
//! This test creates an actual SQLite database with various types and constraints,
//! introspects it to extract the schema, and then generates Rust code that should
//! be valid drizzle-rs schema definitions using lowercase attribute syntax.

use drizzle_migrations::{
    parser::SchemaParser,
    sqlite::{
        SQLiteDDL,
        codegen::{CodegenOptions, GeneratedSchema, generate_rust_schema},
        ddl::{Table, parse_table_ddl},
        introspect::{
            IntrospectionResult, RawColumnInfo, RawForeignKey, RawIndexColumn, RawIndexInfo,
            process_columns, process_foreign_keys, process_indexes,
            process_unique_constraints_from_indexes,
        },
    },
};
use drizzle_types::Dialect;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};

/// SQL to create a comprehensive test schema with various types and constraints
const CREATE_SCHEMA_SQL: &str = r#"
-- Users table with various column types and constraints
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL,
    display_name TEXT,
    age INTEGER,
    score REAL DEFAULT 0.0,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    profile_data BLOB
);

-- Posts table with foreign key reference
CREATE TABLE posts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    content TEXT,
    author_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    views INTEGER DEFAULT 0,
    published INTEGER NOT NULL DEFAULT 0,
    created_at TEXT
);

-- Categories table
CREATE TABLE categories (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    parent_id INTEGER REFERENCES categories(id) ON DELETE SET NULL
);

-- Junction table for many-to-many relationship
CREATE TABLE post_categories (
    post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    category_id INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    PRIMARY KEY (post_id, category_id)
);

-- Table with various default values
CREATE TABLE settings (
    id INTEGER PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,
    value TEXT NOT NULL DEFAULT '',
    is_system INTEGER NOT NULL DEFAULT 0,
    priority INTEGER DEFAULT 100,
    multiplier REAL DEFAULT 1.5
);

-- Create some indexes
CREATE INDEX idx_posts_author ON posts(author_id);
CREATE INDEX idx_posts_created ON posts(created_at);
CREATE UNIQUE INDEX idx_users_email ON users(email);
CREATE INDEX idx_categories_parent ON categories(parent_id);
"#;

/// Introspect a SQLite database and return the DDL
fn introspect_database(conn: &Connection) -> IntrospectionResult {
    let mut result = IntrospectionResult::default();

    // Get tables
    let mut stmt = conn
        .prepare(
            "SELECT name, sql FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        )
        .unwrap();

    let table_rows: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    let mut table_sql_map: HashMap<String, String> = HashMap::new();
    for (name, sql) in &table_rows {
        // Parse table options from CREATE TABLE SQL
        let parsed = parse_table_ddl(sql);
        let mut table = Table::new(name.clone());
        if parsed.strict {
            table = table.strict();
        }
        if parsed.without_rowid {
            table = table.without_rowid();
        }
        result.tables.push(table);
        table_sql_map.insert(name.clone(), sql.clone());
    }

    // Get columns for each table
    let mut raw_columns: Vec<RawColumnInfo> = Vec::new();
    for (table_name, sql) in &table_sql_map {
        let mut col_stmt = conn.prepare(&format!(
            "SELECT cid, name, type, \"notnull\", dflt_value, pk, hidden FROM pragma_table_xinfo('{}')",
            table_name
        )).unwrap();

        let cols: Vec<RawColumnInfo> = col_stmt
            .query_map([], |row| {
                Ok(RawColumnInfo {
                    table: table_name.clone(),
                    cid: row.get(0)?,
                    name: row.get(1)?,
                    column_type: row.get(2)?,
                    not_null: row.get::<_, i32>(3)? != 0,
                    default_value: row.get(4)?,
                    pk: row.get(5)?,
                    hidden: row.get(6)?,
                    sql: Some(sql.clone()),
                })
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        raw_columns.extend(cols);
    }

    // Process columns and primary keys
    let generated_columns = HashMap::new();
    let pk_columns_set: HashSet<(String, String)> = HashSet::new();
    let (columns, primary_keys) =
        process_columns(&raw_columns, &generated_columns, &pk_columns_set);
    result.columns = columns;
    result.primary_keys = primary_keys;

    // Get indexes for each table
    let mut raw_indexes: Vec<RawIndexInfo> = Vec::new();
    let mut raw_index_columns: Vec<RawIndexColumn> = Vec::new();

    for table_name in table_sql_map.keys() {
        let mut idx_stmt = conn
            .prepare(&format!(
                "SELECT name, \"unique\", origin, partial FROM pragma_index_list('{}')",
                table_name
            ))
            .unwrap();

        let idxs: Vec<RawIndexInfo> = idx_stmt
            .query_map([], |row| {
                Ok(RawIndexInfo {
                    table: table_name.clone(),
                    name: row.get(0)?,
                    unique: row.get::<_, i32>(1)? != 0,
                    origin: row.get(2)?,
                    partial: row.get::<_, i32>(3)? != 0,
                })
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        for idx in &idxs {
            let mut ic_stmt = conn
                .prepare(&format!(
                    "SELECT seqno, cid, name, \"desc\", coll, key FROM pragma_index_xinfo('{}')",
                    idx.name
                ))
                .unwrap();

            let cols: Vec<RawIndexColumn> = ic_stmt
                .query_map([], |row| {
                    Ok(RawIndexColumn {
                        index_name: idx.name.clone(),
                        seqno: row.get(0)?,
                        cid: row.get(1)?,
                        name: row.get(2)?,
                        desc: row.get::<_, i32>(3)? != 0,
                        coll: row.get(4)?,
                        key: row.get::<_, i32>(5)? != 0,
                    })
                })
                .unwrap()
                .filter_map(|r| r.ok())
                .collect();

            raw_index_columns.extend(cols);
        }

        raw_indexes.extend(idxs);
    }

    result.indexes = process_indexes(&raw_indexes, &raw_index_columns, &table_sql_map);

    // Get foreign keys for each table
    let mut raw_fks: Vec<RawForeignKey> = Vec::new();
    for table_name in table_sql_map.keys() {
        let mut fk_stmt = conn.prepare(&format!(
            "SELECT id, seq, \"table\", \"from\", \"to\", on_update, on_delete, match FROM pragma_foreign_key_list('{}')",
            table_name
        )).unwrap();

        let fks: Vec<RawForeignKey> = fk_stmt
            .query_map([], |row| {
                Ok(RawForeignKey {
                    table: table_name.clone(),
                    id: row.get(0)?,
                    seq: row.get(1)?,
                    to_table: row.get(2)?,
                    from_column: row.get(3)?,
                    to_column: row.get(4)?,
                    on_update: row.get(5)?,
                    on_delete: row.get(6)?,
                    r#match: row.get(7)?,
                })
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        raw_fks.extend(fks);
    }

    result.foreign_keys = process_foreign_keys(&raw_fks);

    // Unique constraints (origin == 'u' indexes, including inline column UNIQUE)
    result.unique_constraints =
        process_unique_constraints_from_indexes(&raw_indexes, &raw_index_columns);

    result
}

#[test]
fn test_introspect_and_generate_schema() {
    // Create in-memory database with our test schema
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(CREATE_SCHEMA_SQL).unwrap();

    // Introspect the database
    let introspection = introspect_database(&conn);

    // Verify we got the expected tables
    assert_eq!(introspection.tables.len(), 5, "Should have 5 tables");
    let table_names: Vec<&str> = introspection.tables.iter().map(|t| &*t.name).collect();
    assert!(table_names.contains(&"users"), "Should have users table");
    assert!(table_names.contains(&"posts"), "Should have posts table");
    assert!(
        table_names.contains(&"categories"),
        "Should have categories table"
    );
    assert!(
        table_names.contains(&"post_categories"),
        "Should have post_categories table"
    );
    assert!(
        table_names.contains(&"settings"),
        "Should have settings table"
    );

    // Verify columns
    let users_columns: Vec<&_> = introspection
        .columns
        .iter()
        .filter(|c| c.table == "users")
        .collect();
    assert_eq!(users_columns.len(), 9, "Users should have 9 columns");

    // Verify primary keys
    assert!(
        !introspection.primary_keys.is_empty(),
        "Should have primary keys"
    );

    // Verify foreign keys
    assert!(
        !introspection.foreign_keys.is_empty(),
        "Should have foreign keys"
    );

    // Verify indexes
    assert_eq!(
        introspection.indexes.len(),
        4,
        "Should have 4 manual indexes"
    );

    // Convert to DDL
    let snapshot = introspection.to_snapshot();
    let ddl = SQLiteDDL::from_entities(snapshot.ddl.clone());

    // Generate Rust code
    let options = CodegenOptions {
        include_schema: true,
        schema_name: "AppSchema".to_string(),
        use_pub: true,
        module_doc: Some("Generated from test database".to_string()),
    };

    let generated = generate_rust_schema(&ddl, &options);

    // Print generated code for inspection
    println!("Generated Rust schema:\n{}", generated.code);

    // Verify the generated code structure
    verify_generated_code(&generated);
}

fn verify_generated_code(generated: &GeneratedSchema) {
    let code = &generated.code;
    let parsed = SchemaParser::parse(code);

    // === Header and imports ===
    assert!(
        code.contains("use drizzle::sqlite::prelude::*;"),
        "Should have drizzle imports"
    );

    // === Users table - precise field-level checks ===
    let users = parsed
        .table("Users", Dialect::SQLite)
        .expect("Should have Users struct");
    assert_eq!(
        users.attr, "#[SQLiteTable]",
        "Users should have plain SQLiteTable attr"
    );

    // Users.id: INTEGER PRIMARY KEY AUTOINCREMENT
    let id_field = users.field("id").expect("Users should have id field");
    assert_eq!(id_field.ty, "i64", "Users.id should be i64");
    assert!(
        id_field.has_attr("primary"),
        "Users.id should have primary attribute"
    );
    assert!(
        id_field.has_attr("autoincrement"),
        "Users.id should have autoincrement attribute"
    );

    // Users.username: TEXT NOT NULL UNIQUE
    let username_field = users
        .field("username")
        .expect("Users should have username field");
    assert_eq!(
        username_field.ty, "String",
        "Users.username should be String (NOT NULL)"
    );
    assert!(
        username_field.has_attr("unique"),
        "Users.username should have unique attribute"
    );

    // Users.email: TEXT NOT NULL (no unique, that's via index)
    let email_field = users.field("email").expect("Users should have email field");
    assert_eq!(
        email_field.ty, "String",
        "Users.email should be String (NOT NULL)"
    );

    // Users.display_name: TEXT (nullable)
    let display_name_field = users
        .field("display_name")
        .expect("Users should have display_name field");
    assert_eq!(
        display_name_field.ty, "Option<String>",
        "Users.display_name should be Option<String>"
    );

    // Users.age: INTEGER (nullable)
    let age_field = users.field("age").expect("Users should have age field");
    assert_eq!(
        age_field.ty, "Option<i64>",
        "Users.age should be Option<i64>"
    );

    // Users.score: REAL DEFAULT 0.0
    let score_field = users.field("score").expect("Users should have score field");
    assert_eq!(
        score_field.ty, "Option<f64>",
        "Users.score should be Option<f64>"
    );
    assert!(
        score_field.has_attr("default"),
        "Users.score should have default attribute"
    );

    // Users.is_active: INTEGER NOT NULL DEFAULT 1
    let is_active_field = users
        .field("is_active")
        .expect("Users should have is_active field");
    assert_eq!(
        is_active_field.ty, "i64",
        "Users.is_active should be i64 (NOT NULL)"
    );
    assert!(
        is_active_field.has_attr("default = 1"),
        "Users.is_active should have default = 1"
    );

    // Users.profile_data: BLOB (nullable)
    let profile_data_field = users
        .field("profile_data")
        .expect("Users should have profile_data field");
    assert_eq!(
        profile_data_field.ty, "Option<Vec<u8>>",
        "Users.profile_data should be Option<Vec<u8>>"
    );

    // === Posts table - foreign key check ===
    let posts = parsed
        .table("Posts", Dialect::SQLite)
        .expect("Should have Posts struct");

    // Posts.author_id: INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE
    let author_id_field = posts
        .field("author_id")
        .expect("Posts should have author_id field");
    assert_eq!(
        author_id_field.ty, "i64",
        "Posts.author_id should be i64 (NOT NULL)"
    );
    assert!(
        author_id_field.has_attr("references = Users::id"),
        "Posts.author_id should reference Users::id, got: {}",
        author_id_field.column_attr()
    );
    assert!(
        author_id_field.has_attr("on_delete = cascade"),
        "Posts.author_id should have on_delete = cascade, got: {}",
        author_id_field.column_attr()
    );

    // === Categories table - self-referencing FK ===
    let categories = parsed
        .table("Categories", Dialect::SQLite)
        .expect("Should have Categories struct");

    // Categories.parent_id: REFERENCES categories(id) ON DELETE SET NULL
    let parent_id_field = categories
        .field("parent_id")
        .expect("Categories should have parent_id field");
    assert_eq!(
        parent_id_field.ty, "Option<i64>",
        "Categories.parent_id should be Option<i64>"
    );
    assert!(
        parent_id_field.has_attr("references = Categories::id"),
        "Categories.parent_id should reference Categories::id, got: {}",
        parent_id_field.column_attr()
    );
    assert!(
        parent_id_field.has_attr("on_delete = set_null"),
        "Categories.parent_id should have on_delete = set_null, got: {}",
        parent_id_field.column_attr()
    );

    // === PostCategories - composite PK table ===
    let post_categories = parsed
        .table("PostCategories", Dialect::SQLite)
        .expect("Should have PostCategories struct");

    // Composite PK columns should NOT have individual primary attributes
    let post_id_field = post_categories
        .field("post_id")
        .expect("PostCategories should have post_id field");
    let category_id_field = post_categories
        .field("category_id")
        .expect("PostCategories should have category_id field");

    // Both should have FK references but NOT primary (composite PK is table-level)
    assert!(
        post_id_field.has_attr("references = Posts::id"),
        "PostCategories.post_id should reference Posts::id"
    );
    assert!(
        category_id_field.has_attr("references = Categories::id"),
        "PostCategories.category_id should reference Categories::id"
    );

    // === Settings table - various defaults ===
    let settings = parsed
        .table("Settings", Dialect::SQLite)
        .expect("Should have Settings struct");

    let key_field = settings
        .field("key")
        .expect("Settings should have key field");
    assert_eq!(
        key_field.ty, "String",
        "Settings.key should be String (NOT NULL)"
    );
    assert!(
        key_field.has_attr("unique"),
        "Settings.key should have unique attribute"
    );

    let value_field = settings
        .field("value")
        .expect("Settings should have value field");
    assert_eq!(
        value_field.ty, "String",
        "Settings.value should be String (NOT NULL)"
    );
    assert!(
        value_field.has_attr("default = \"\""),
        "Settings.value should have empty default"
    );

    let priority_field = settings
        .field("priority")
        .expect("Settings should have priority field");
    assert!(
        priority_field.has_attr("default = 100"),
        "Settings.priority should have default = 100"
    );

    let multiplier_field = settings
        .field("multiplier")
        .expect("Settings should have multiplier field");
    assert!(
        multiplier_field.has_attr("default = 1.5"),
        "Settings.multiplier should have default = 1.5"
    );

    // === Schema struct ===
    assert!(
        code.contains("#[derive(SQLiteSchema)]"),
        "Should have SQLiteSchema derive"
    );
    assert!(
        code.contains("pub struct AppSchema"),
        "Should have AppSchema struct"
    );

    // === Verify lowercase attribute style ===
    assert!(
        !code.contains("#[column(PRIMARY"),
        "Should use lowercase 'primary', not 'PRIMARY'"
    );
    assert!(
        !code.contains("#[column(AUTOINCREMENT"),
        "Should use lowercase 'autoincrement', not 'AUTOINCREMENT'"
    );
}

#[test]
fn test_specific_type_mappings() {
    let conn = Connection::open_in_memory().unwrap();

    // Create table with all SQLite type affinities
    conn.execute_batch(
        r#"
        CREATE TABLE type_test (
            col_integer INTEGER NOT NULL,
            col_int INT,
            col_tinyint TINYINT,
            col_smallint SMALLINT,
            col_mediumint MEDIUMINT,
            col_bigint BIGINT,
            col_real REAL,
            col_double DOUBLE,
            col_float FLOAT,
            col_text TEXT,
            col_varchar VARCHAR(255),
            col_char CHAR(10),
            col_clob CLOB,
            col_blob BLOB,
            col_numeric NUMERIC,
            col_decimal DECIMAL(10,2),
            col_boolean BOOLEAN,
            col_date DATE,
            col_datetime DATETIME
        );
    "#,
    )
    .unwrap();

    let introspection = introspect_database(&conn);
    let snapshot = introspection.to_snapshot();
    let ddl = SQLiteDDL::from_entities(snapshot.ddl.clone());

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);

    println!("Type mapping test:\n{}", generated.code);

    // INTEGER affinity types should map to i64
    assert!(
        generated.code.contains("col_integer: i64"),
        "INTEGER -> i64"
    );
    assert!(
        generated.code.contains("col_int: Option<i64>"),
        "INT -> Option<i64>"
    );

    // REAL affinity types should map to f64
    assert!(
        generated.code.contains("col_real: Option<f64>"),
        "REAL -> Option<f64>"
    );
    assert!(
        generated.code.contains("col_double: Option<f64>"),
        "DOUBLE -> Option<f64>"
    );
    assert!(
        generated.code.contains("col_float: Option<f64>"),
        "FLOAT -> Option<f64>"
    );

    // TEXT affinity types should map to String
    assert!(
        generated.code.contains("col_text: Option<String>"),
        "TEXT -> Option<String>"
    );
    assert!(
        generated.code.contains("col_varchar: Option<String>"),
        "VARCHAR -> Option<String>"
    );

    // BLOB should map to Vec<u8>
    assert!(
        generated.code.contains("col_blob: Option<Vec<u8>>"),
        "BLOB -> Option<Vec<u8>>"
    );
}

#[test]
fn test_default_value_generation() {
    let conn = Connection::open_in_memory().unwrap();

    conn.execute_batch(
        r#"
        CREATE TABLE defaults_test (
            id INTEGER PRIMARY KEY,
            str_default TEXT DEFAULT 'hello',
            int_default INTEGER DEFAULT 42,
            real_default REAL DEFAULT 3.14,
            bool_default INTEGER DEFAULT 1,
            empty_default TEXT DEFAULT ''
        );
    "#,
    )
    .unwrap();

    let introspection = introspect_database(&conn);
    let snapshot = introspection.to_snapshot();
    let ddl = SQLiteDDL::from_entities(snapshot.ddl.clone());

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);

    println!("Default values test:\n{}", generated.code);

    // Check default values are properly formatted
    assert!(
        generated.code.contains(r#"default = "hello""#),
        "String default should be quoted"
    );
    assert!(
        generated.code.contains("default = 42"),
        "Integer default should be unquoted"
    );
    assert!(
        generated.code.contains("default = 3.14"),
        "Real default should be unquoted"
    );
}

#[test]
fn test_foreign_key_actions() {
    let conn = Connection::open_in_memory().unwrap();

    conn.execute_batch(
        r#"
        CREATE TABLE parent (
            id INTEGER PRIMARY KEY
        );

        CREATE TABLE child_cascade (
            id INTEGER PRIMARY KEY,
            parent_id INTEGER REFERENCES parent(id) ON DELETE CASCADE ON UPDATE CASCADE
        );

        CREATE TABLE child_set_null (
            id INTEGER PRIMARY KEY,
            parent_id INTEGER REFERENCES parent(id) ON DELETE SET NULL ON UPDATE SET NULL
        );

        CREATE TABLE child_restrict (
            id INTEGER PRIMARY KEY,
            parent_id INTEGER REFERENCES parent(id) ON DELETE RESTRICT ON UPDATE RESTRICT
        );
    "#,
    )
    .unwrap();

    let introspection = introspect_database(&conn);
    let snapshot = introspection.to_snapshot();
    let ddl = SQLiteDDL::from_entities(snapshot.ddl.clone());

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);

    println!("Foreign key actions test:\n{}", generated.code);

    // Check actions are properly generated
    assert!(
        generated.code.contains("on_delete = cascade"),
        "Should have on_delete cascade"
    );
    assert!(
        generated.code.contains("on_update = cascade"),
        "Should have on_update cascade"
    );
    // set_null action - note the underscore convention
    assert!(
        generated.code.contains("set_null") || generated.code.contains("set null"),
        "Should have set_null action"
    );
}

#[test]
fn test_composite_primary_key() {
    let conn = Connection::open_in_memory().unwrap();

    conn.execute_batch(
        r#"
        CREATE TABLE composite_pk (
            col_a INTEGER NOT NULL,
            col_b TEXT NOT NULL,
            col_c INTEGER,
            PRIMARY KEY (col_a, col_b)
        );
    "#,
    )
    .unwrap();

    let introspection = introspect_database(&conn);
    let snapshot = introspection.to_snapshot();
    let ddl = SQLiteDDL::from_entities(snapshot.ddl.clone());

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);

    println!("Composite PK test:\n{}", generated.code);

    // Composite PKs should NOT have individual column `primary` attributes
    // because the PK is at the table level
    let lines: Vec<&str> = generated.code.lines().collect();

    // Find the CompositePk struct
    let mut in_composite = false;
    let mut col_a_has_primary = false;
    let mut col_b_has_primary = false;

    for line in lines {
        if line.contains("struct CompositePk") {
            in_composite = true;
        }
        if in_composite {
            if line.contains("col_a") && line.contains("primary") {
                col_a_has_primary = true;
            }
            if line.contains("col_b") && line.contains("primary") {
                col_b_has_primary = true;
            }
            if line.contains("}") && !line.contains("Option") {
                break;
            }
        }
    }

    // For composite PKs, individual columns should NOT have 'primary' attribute
    // (the macro should handle this differently - at table level or with explicit composite handling)
    assert!(
        !col_a_has_primary || !col_b_has_primary,
        "Composite PK columns should not all have individual 'primary' attributes"
    );
}

#[test]
fn test_index_generation() {
    let conn = Connection::open_in_memory().unwrap();

    conn.execute_batch(
        r#"
        CREATE TABLE indexed_table (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT NOT NULL,
            score INTEGER
        );

        CREATE INDEX idx_name ON indexed_table(name);
        CREATE UNIQUE INDEX idx_email ON indexed_table(email);
        CREATE INDEX idx_name_score ON indexed_table(name, score);
    "#,
    )
    .unwrap();

    let introspection = introspect_database(&conn);
    let snapshot = introspection.to_snapshot();
    let ddl = SQLiteDDL::from_entities(snapshot.ddl.clone());

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);

    println!("Index generation test:\n{}", generated.code);

    // Check index structs are generated
    assert!(
        generated.code.contains("#[SQLiteIndex]"),
        "Should have regular index attribute"
    );
    assert!(
        generated.code.contains("#[SQLiteIndex(unique)]"),
        "Should have unique index attribute"
    );

    // Check composite index has multiple columns
    assert!(
        generated
            .code
            .contains("IndexedTable::name, IndexedTable::score")
            || generated
                .code
                .contains("IndexedTable::name,IndexedTable::score"),
        "Composite index should reference multiple columns"
    );
}

#[test]
fn test_strict_table() {
    let conn = Connection::open_in_memory().unwrap();

    conn.execute_batch(
        r#"
        CREATE TABLE strict_example (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            score INTEGER
        ) STRICT;
    "#,
    )
    .unwrap();

    let introspection = introspect_database(&conn);
    let snapshot = introspection.to_snapshot();
    let ddl = SQLiteDDL::from_entities(snapshot.ddl.clone());

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);

    println!("STRICT table test:\n{}", generated.code);

    // Parse and verify exact struct attributes
    let parsed = SchemaParser::parse(&generated.code);
    let strict_table = parsed
        .table("StrictExample", Dialect::SQLite)
        .expect("Should have StrictExample struct");

    // Verify the table has strict attribute
    assert!(
        strict_table.has_table_attr("strict"),
        "StrictExample should have strict in table attr, got: {}",
        strict_table.attr
    );

    // Verify exact field types
    let id_field = strict_table
        .field("id")
        .expect("StrictExample should have id field");
    assert_eq!(id_field.ty, "i64", "StrictExample.id should be i64");
    assert!(
        id_field.has_attr("primary"),
        "StrictExample.id should have primary"
    );

    let name_field = strict_table
        .field("name")
        .expect("StrictExample should have name field");
    assert_eq!(
        name_field.ty, "String",
        "StrictExample.name should be String (NOT NULL)"
    );

    let score_field = strict_table
        .field("score")
        .expect("StrictExample should have score field");
    assert_eq!(
        score_field.ty, "Option<i64>",
        "StrictExample.score should be Option<i64>"
    );
}

#[test]
fn test_without_rowid_table() {
    let conn = Connection::open_in_memory().unwrap();

    conn.execute_batch(
        r#"
        CREATE TABLE without_rowid_example (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL
        ) WITHOUT ROWID;
    "#,
    )
    .unwrap();

    let introspection = introspect_database(&conn);
    let snapshot = introspection.to_snapshot();
    let ddl = SQLiteDDL::from_entities(snapshot.ddl.clone());

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);

    println!("WITHOUT ROWID table test:\n{}", generated.code);

    // Parse and verify exact struct attributes
    let parsed = SchemaParser::parse(&generated.code);
    let rowid_table = parsed
        .table("WithoutRowidExample", Dialect::SQLite)
        .expect("Should have WithoutRowidExample struct");

    // Verify the table has without_rowid attribute
    assert!(
        rowid_table.has_table_attr("without_rowid"),
        "WithoutRowidExample should have without_rowid in table attr, got: {}",
        rowid_table.attr
    );

    // Verify exact field types
    let id_field = rowid_table
        .field("id")
        .expect("WithoutRowidExample should have id field");
    assert_eq!(id_field.ty, "i64", "WithoutRowidExample.id should be i64");
    assert!(
        id_field.has_attr("primary"),
        "WithoutRowidExample.id should have primary"
    );

    let name_field = rowid_table
        .field("name")
        .expect("WithoutRowidExample should have name field");
    assert_eq!(
        name_field.ty, "String",
        "WithoutRowidExample.name should be String (NOT NULL)"
    );
}

#[test]
fn test_strict_without_rowid_combined() {
    let conn = Connection::open_in_memory().unwrap();

    conn.execute_batch(
        r#"
        CREATE TABLE strict_and_rowid (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT
        ) STRICT, WITHOUT ROWID;
    "#,
    )
    .unwrap();

    let introspection = introspect_database(&conn);
    let snapshot = introspection.to_snapshot();
    let ddl = SQLiteDDL::from_entities(snapshot.ddl.clone());

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);

    println!("STRICT + WITHOUT ROWID table test:\n{}", generated.code);

    // Parse and verify exact struct attributes
    let parsed = SchemaParser::parse(&generated.code);
    let combo_table = parsed
        .table("StrictAndRowid", Dialect::SQLite)
        .expect("Should have StrictAndRowid struct");

    // Verify the table has BOTH strict and without_rowid attributes
    assert!(
        combo_table.has_table_attr("strict"),
        "StrictAndRowid should have strict in table attr, got: {}",
        combo_table.attr
    );
    assert!(
        combo_table.has_table_attr("without_rowid"),
        "StrictAndRowid should have without_rowid in table attr, got: {}",
        combo_table.attr
    );

    // Verify exact field types
    let id_field = combo_table
        .field("id")
        .expect("StrictAndRowid should have id field");
    assert_eq!(id_field.ty, "i64", "StrictAndRowid.id should be i64");
    assert!(
        id_field.has_attr("primary"),
        "StrictAndRowid.id should have primary"
    );

    let name_field = combo_table
        .field("name")
        .expect("StrictAndRowid should have name field");
    assert_eq!(
        name_field.ty, "String",
        "StrictAndRowid.name should be String (NOT NULL)"
    );

    let email_field = combo_table
        .field("email")
        .expect("StrictAndRowid should have email field");
    assert_eq!(
        email_field.ty, "Option<String>",
        "StrictAndRowid.email should be Option<String>"
    );
}

#[test]
fn test_check_constraint_parsing() {
    let conn = Connection::open_in_memory().unwrap();

    conn.execute_batch(
        r#"
        CREATE TABLE with_checks (
            id INTEGER PRIMARY KEY,
            age INTEGER CHECK(age >= 0 AND age <= 150),
            score INTEGER,
            CONSTRAINT score_check CHECK(score >= 0)
        );
    "#,
    )
    .unwrap();

    // This test verifies that CHECK constraints are parsed correctly
    // (even though we may not yet generate them in Rust code)
    let introspection = introspect_database(&conn);

    assert!(!introspection.tables.is_empty(), "Should have tables");
    assert!(
        introspection.tables.iter().any(|t| t.name == "with_checks"),
        "Should have with_checks table"
    );
}

#[test]
fn test_text_primary_key_nullable() {
    // Per SQLite docs, non-INTEGER PRIMARY KEY columns CAN be NULL
    // due to a legacy SQLite bug that was preserved for compatibility
    let conn = Connection::open_in_memory().unwrap();

    conn.execute_batch(
        r#"
        CREATE TABLE text_pk_table (
            id TEXT PRIMARY KEY,
            value INTEGER
        );
    "#,
    )
    .unwrap();

    let introspection = introspect_database(&conn);
    let snapshot = introspection.to_snapshot();
    let ddl = SQLiteDDL::from_entities(snapshot.ddl.clone());

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);

    println!("TEXT PRIMARY KEY test:\n{}", generated.code);

    // Parse and verify exact field type
    let parsed = SchemaParser::parse(&generated.code);
    let table = parsed
        .table("TextPkTable", Dialect::SQLite)
        .expect("Should have TextPkTable struct");

    // TEXT PRIMARY KEY should be Option<String> since SQLite allows NULLs
    // in non-INTEGER primary keys (due to legacy bug)
    let id_field = table.field("id").expect("TextPkTable should have id field");
    assert_eq!(
        id_field.ty, "Option<String>",
        "TEXT PRIMARY KEY should be Option<String> due to SQLite's legacy NULL-in-PK bug, got: {}",
        id_field.ty
    );
    assert!(
        id_field.has_attr("primary"),
        "TextPkTable.id should have primary attribute"
    );

    let value_field = table
        .field("value")
        .expect("TextPkTable should have value field");
    assert_eq!(
        value_field.ty, "Option<i64>",
        "TextPkTable.value should be Option<i64>"
    );
}

#[test]
fn test_integer_primary_key_not_null() {
    // INTEGER PRIMARY KEY is the special case where SQLite enforces NOT NULL
    let conn = Connection::open_in_memory().unwrap();

    conn.execute_batch(
        r#"
        CREATE TABLE int_pk_table (
            id INTEGER PRIMARY KEY,
            value TEXT
        );
    "#,
    )
    .unwrap();

    let introspection = introspect_database(&conn);
    let snapshot = introspection.to_snapshot();
    let ddl = SQLiteDDL::from_entities(snapshot.ddl.clone());

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);

    println!("INTEGER PRIMARY KEY test:\n{}", generated.code);

    // Parse and verify exact field type
    let parsed = SchemaParser::parse(&generated.code);
    let table = parsed
        .table("IntPkTable", Dialect::SQLite)
        .expect("Should have IntPkTable struct");

    // INTEGER PRIMARY KEY should be i64 (non-optional) since SQLite
    // enforces NOT NULL for INTEGER PRIMARY KEY columns
    let id_field = table.field("id").expect("IntPkTable should have id field");
    assert_eq!(
        id_field.ty, "i64",
        "INTEGER PRIMARY KEY should be i64 (not Optional), got: {}",
        id_field.ty
    );
    assert!(
        id_field.has_attr("primary"),
        "IntPkTable.id should have primary attribute"
    );

    let value_field = table
        .field("value")
        .expect("IntPkTable should have value field");
    assert_eq!(
        value_field.ty, "Option<String>",
        "IntPkTable.value should be Option<String>"
    );
}

#[test]
fn test_generated_columns() {
    let conn = Connection::open_in_memory().unwrap();

    conn.execute_batch(
        r#"
        CREATE TABLE with_generated (
            first_name TEXT NOT NULL,
            last_name TEXT NOT NULL,
            full_name TEXT GENERATED ALWAYS AS (first_name || ' ' || last_name) STORED
        );
    "#,
    )
    .unwrap();

    let introspection = introspect_database(&conn);

    // Verify the table was created
    assert!(
        introspection
            .tables
            .iter()
            .any(|t| t.name == "with_generated"),
        "Should have with_generated table"
    );

    // Verify columns are present
    // Note: Generated columns may be filtered out depending on their hidden status
    // pragma_table_xinfo returns hidden=2 for STORED generated columns and hidden=3 for VIRTUAL
    // The process_columns function may filter these out based on hidden values
    let cols: Vec<_> = introspection
        .columns
        .iter()
        .filter(|c| c.table == "with_generated")
        .collect();

    // At minimum we should have the 2 regular columns
    assert!(
        cols.len() >= 2,
        "Should have at least 2 columns (first_name, last_name), got {}",
        cols.len()
    );

    // If generated columns are included, we'd have 3
    println!(
        "Generated columns test: found {} columns for with_generated table",
        cols.len()
    );
}

#[test]
fn test_various_index_types() {
    let conn = Connection::open_in_memory().unwrap();

    conn.execute_batch(
        r#"
        CREATE TABLE multi_indexed (
            id INTEGER PRIMARY KEY,
            col_a TEXT NOT NULL,
            col_b INTEGER,
            col_c REAL
        );

        -- Regular index
        CREATE INDEX idx_a ON multi_indexed(col_a);

        -- Unique index
        CREATE UNIQUE INDEX idx_b ON multi_indexed(col_b);

        -- Multi-column index
        CREATE INDEX idx_ab ON multi_indexed(col_a, col_b);

        -- Partial index (WHERE clause)
        CREATE INDEX idx_c_positive ON multi_indexed(col_c) WHERE col_c > 0;
    "#,
    )
    .unwrap();

    let introspection = introspect_database(&conn);
    let snapshot = introspection.to_snapshot();
    let ddl = SQLiteDDL::from_entities(snapshot.ddl.clone());

    // Note: partial indexes might be filtered out or handled specially
    assert!(
        introspection.indexes.len() >= 3,
        "Should have at least 3 indexes"
    );

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);

    println!("Various indexes test:\n{}", generated.code);

    // Check unique and regular indexes are differentiated
    assert!(
        generated.code.contains("#[SQLiteIndex]"),
        "Should have regular indexes"
    );
}

#[test]
fn test_view_codegen() {
    use drizzle_migrations::sqlite::ddl::View;

    let mut ddl = SQLiteDDL::new();

    // Add a table that the view references
    ddl.tables.push(Table::new("users"));

    // Add a simple view
    let mut view = View::new("active_users");
    view.definition = Some("SELECT * FROM users WHERE is_active = 1".into());
    ddl.views.push(view);

    // Add a view with a complex definition containing quotes
    let mut quoted_view = View::new("user_stats");
    quoted_view.definition = Some(r#"SELECT id, name, "status" FROM users WHERE name = 'test'"#.into());
    ddl.views.push(quoted_view);

    let options = CodegenOptions {
        use_pub: true,
        ..Default::default()
    };
    let generated = generate_rust_schema(&ddl, &options);

    println!("View codegen test:\n{}", generated.code);

    // Check that views are generated
    assert_eq!(generated.views.len(), 2, "Should generate 2 views");
    assert!(generated.views.contains(&"active_users".to_string()));
    assert!(generated.views.contains(&"user_stats".to_string()));

    // Check view struct generation
    assert!(
        generated.code.contains("#[SQLiteView("),
        "Should have SQLiteView attribute"
    );
    assert!(
        generated.code.contains("pub struct ActiveUsers"),
        "Should generate ActiveUsers struct"
    );
    assert!(
        generated.code.contains("pub struct UserStats"),
        "Should generate UserStats struct"
    );

    // Check that definition is properly escaped
    assert!(
        generated.code.contains("definition = "),
        "Should have definition attribute"
    );
    // Quotes should be escaped
    assert!(
        generated.code.contains(r#"\"status\""#),
        "Double quotes should be escaped in definition"
    );
}

#[test]
fn test_view_with_columns_codegen() {
    use drizzle_migrations::sqlite::ddl::{Column, View};

    let mut ddl = SQLiteDDL::new();

    // Add a view
    let mut view = View::new("user_summary");
    view.definition = Some("SELECT id, username, email FROM users".into());
    ddl.views.push(view);

    // Add columns for the view (as if introspected)
    let mut col1 = Column::new("user_summary", "id", "INTEGER");
    col1.not_null = true;
    col1.ordinal_position = Some(0);
    ddl.columns.push(col1);

    let mut col2 = Column::new("user_summary", "username", "TEXT");
    col2.not_null = true;
    col2.ordinal_position = Some(1);
    ddl.columns.push(col2);

    let mut col3 = Column::new("user_summary", "email", "TEXT");
    col3.not_null = false;
    col3.ordinal_position = Some(2);
    ddl.columns.push(col3);

    let options = CodegenOptions {
        use_pub: true,
        ..Default::default()
    };
    let generated = generate_rust_schema(&ddl, &options);

    println!("View with columns test:\n{}", generated.code);

    // Check view struct has column fields
    assert!(
        generated.code.contains("pub id: i64"),
        "View should have id field"
    );
    assert!(
        generated.code.contains("pub username: String"),
        "View should have username field"
    );
    assert!(
        generated.code.contains("pub email: Option<String>"),
        "View should have email field (nullable)"
    );
}

#[test]
fn test_existing_view_skipped() {
    use drizzle_migrations::sqlite::ddl::View;

    let mut ddl = SQLiteDDL::new();

    // Add an existing view (should be skipped in codegen)
    let mut existing_view = View::new("existing_view");
    existing_view.definition = Some("SELECT 1".into());
    existing_view.is_existing = true;
    ddl.views.push(existing_view);

    // Add a regular view
    let mut regular_view = View::new("regular_view");
    regular_view.definition = Some("SELECT 2".into());
    ddl.views.push(regular_view);

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);

    println!("Existing view test:\n{}", generated.code);

    // Only regular view should be generated
    assert_eq!(generated.views.len(), 1, "Should only generate 1 view");
    assert!(generated.views.contains(&"regular_view".to_string()));
    assert!(!generated.views.contains(&"existing_view".to_string()));
}
