//! End-to-end regression tests for the syn-based schema parser.
//!
//! Each test pins one of the defects the hand-rolled scanner had (P1-P16 in
//! the rewrite audit): the parser plus `Snapshot::from_parse_result` must
//! produce the same DDL the derive macros would, never silently dropping or
//! inventing schema shape. Finer-grained per-attribute tests live in
//! `migrations/src/parser/attrs.rs`; producer parity against the real macros
//! lives in the root package (`tests/parser_parity.rs`).

use drizzle_migrations::parser::SchemaParser;
use drizzle_migrations::postgres::ddl::PostgresEntity;
use drizzle_migrations::schema::Snapshot;
use drizzle_migrations::sqlite::SqliteEntity;
use drizzle_types::Dialect;

fn sqlite_entities(code: &str) -> Vec<SqliteEntity> {
    let result = SchemaParser::parse(code);
    assert!(
        result.errors.is_empty(),
        "parse errors: {:?}",
        result.errors
    );
    match Snapshot::from_parse_result(&result, Dialect::SQLite, None) {
        Snapshot::Sqlite(s) => s.ddl,
        Snapshot::Postgres(_) => panic!("expected SQLite snapshot"),
    }
}

fn postgres_entities(code: &str) -> Vec<PostgresEntity> {
    let result = SchemaParser::parse(code);
    assert!(
        result.errors.is_empty(),
        "parse errors: {:?}",
        result.errors
    );
    match Snapshot::from_parse_result(&result, Dialect::PostgreSQL, None) {
        Snapshot::Postgres(s) => s.ddl,
        Snapshot::Sqlite(_) => panic!("expected Postgres snapshot"),
    }
}

/// P1: the repo's own uppercase spellings (`PRIMARY`, `REFERENCES = ...`,
/// `DEFAULT_FN`) must not be silently lost.
#[test]
fn uppercase_markers_survive_to_snapshot() {
    let entities = sqlite_entities(
        r#"
#[SQLiteTable]
pub struct Users {
    #[column(PRIMARY, AUTOINCREMENT)]
    pub id: i64,
    #[column(UNIQUE)]
    pub email: String,
    #[column(REFERENCES = Users::id, ON_DELETE = CASCADE)]
    pub invited_by: Option<i64>,
    #[column(DEFAULT_FN = String::new)]
    pub note: String,
}
"#,
    );

    assert!(
        entities
            .iter()
            .any(|e| matches!(e, SqliteEntity::PrimaryKey(pk) if pk.name.as_ref() == "users_pk"))
    );
    assert!(entities.iter().any(|e| matches!(
        e,
        SqliteEntity::Column(c) if c.name.as_ref() == "id" && c.autoincrement == Some(true)
    )));
    assert!(entities.iter().any(
        |e| matches!(e, SqliteEntity::UniqueConstraint(u) if u.name.as_ref() == "users_email_unique")
    ));
    assert!(entities.iter().any(|e| matches!(
        e,
        SqliteEntity::ForeignKey(fk)
            if fk.name.as_ref() == "fk_users_invited_by_users_id_fk"
                && fk.on_delete.as_deref() == Some("CASCADE")
    )));
}

/// P2: substring matches must not produce phantom PRIMARY/UNIQUE/STRICT.
#[test]
fn no_phantom_constraints_from_names_or_strings() {
    let entities = sqlite_entities(
        r#"
#[SQLiteTable(name = "strict_mode")]
pub struct Config {
    #[column(name = "primary_email")]
    pub email: String,
    #[column(default = "unique snowflake")]
    pub label: String,
}
"#,
    );

    let table = entities
        .iter()
        .find_map(|e| match e {
            SqliteEntity::Table(t) => Some(t),
            _ => None,
        })
        .expect("table entity");
    assert_eq!(table.name.as_ref(), "strict_mode");
    assert!(!table.strict, "table name must not imply STRICT");

    assert!(
        !entities
            .iter()
            .any(|e| matches!(e, SqliteEntity::PrimaryKey(_))),
        "column name must not imply PRIMARY KEY"
    );
    assert!(
        !entities
            .iter()
            .any(|e| matches!(e, SqliteEntity::UniqueConstraint(_))),
        "default string must not imply UNIQUE"
    );
}

/// P3: referential actions are stored normalized (`SET_NULL` -> `SET NULL`),
/// never emitted verbatim as invalid SQL.
#[test]
fn referential_actions_are_normalized() {
    let entities = sqlite_entities(
        r#"
#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub id: i64,
}

#[SQLiteTable]
pub struct Posts {
    #[column(primary)]
    pub id: i64,
    #[column(references = Users::id, on_delete = SET_NULL, on_update = no_action)]
    pub author_id: Option<i64>,
}
"#,
    );

    let fk = entities
        .iter()
        .find_map(|e| match e {
            SqliteEntity::ForeignKey(fk) => Some(fk),
            _ => None,
        })
        .expect("fk entity");
    assert_eq!(fk.on_delete.as_deref(), Some("SET NULL"));
    assert_eq!(fk.on_update.as_deref(), Some("NO ACTION"));
}

/// P4 + P9: string defaults are SQL-quoted (not Rust-quoted) and
/// `default_sql` is honored with the macro's normalization.
#[test]
fn defaults_render_like_the_macros() {
    let entities = sqlite_entities(
        r#"
#[SQLiteTable]
pub struct Defaults {
    #[column(default = "hello")]
    pub greeting: String,
    #[column(default_sql = "strftime('%s','now')")]
    pub epoch: i64,
    #[column(default_sql = "CURRENT_TIMESTAMP")]
    pub created: String,
}
"#,
    );

    let default_of = |name: &str| {
        entities
            .iter()
            .find_map(|e| match e {
                SqliteEntity::Column(c) if c.name.as_ref() == name => {
                    Some(c.default.as_deref().map(str::to_string))
                }
                _ => None,
            })
            .expect("column")
    };
    assert_eq!(default_of("greeting").as_deref(), Some("'hello'"));
    assert_eq!(
        default_of("epoch").as_deref(),
        Some("(strftime('%s','now'))")
    );
    assert_eq!(default_of("created").as_deref(), Some("CURRENT_TIMESTAMP"));
}

/// P5 + P7: braces/parens inside attribute strings must not truncate
/// structs or skip fields.
#[test]
fn string_aware_parsing() {
    let entities = sqlite_entities(
        r#"
#[SQLiteTable]
pub struct Tricky {
    #[column(primary)]
    pub id: i64,
    #[column(default = "{}")]
    pub payload: String,
    #[column(check = "instr(x, ')') = 0")]
    pub x: String,
    pub trailing: i64,
}
"#,
    );

    let columns: Vec<&str> = entities
        .iter()
        .filter_map(|e| match e {
            SqliteEntity::Column(c) => Some(c.name.as_ref()),
            _ => None,
        })
        .collect();
    assert_eq!(columns, vec!["id", "payload", "x", "trailing"]);
    assert!(entities.iter().any(|e| matches!(
        e,
        SqliteEntity::CheckConstraint(c) if c.value.as_ref() == "instr(x, ')') = 0"
    )));
}

/// P6: `pub(crate)` fields are columns too.
#[test]
fn restricted_visibility_fields_are_kept() {
    let entities = sqlite_entities(
        r#"
#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub(crate) id: i64,
    pub(super) name: String,
}
"#,
    );
    let columns = entities
        .iter()
        .filter(|e| matches!(e, SqliteEntity::Column(_)))
        .count();
    assert_eq!(columns, 2);
}

/// P8: enums, views, policies, and table-level FOREIGN_KEY/UNIQUE/CHECK all
/// materialize as entities.
#[test]
fn entity_classes_are_emitted() {
    let entities = postgres_entities(
        r#"
#[derive(PostgresEnum, Default, Clone)]
pub enum Status {
    #[default]
    Active,
    Retired,
}

#[PostgresTable(RLS, UNIQUE(columns(a, b)), CHECK(expr = "a > 0"))]
pub struct Things {
    #[column(primary)]
    pub id: i32,
    pub a: i32,
    pub b: i32,
    #[column(enum)]
    pub status: Status,
}

#[PostgresView(definition = "SELECT id FROM things")]
pub struct ThingIds {
    pub id: i32,
}

#[PostgresPolicy(FOR = "SELECT", USING = "true")]
pub struct ThingsPolicy(Things);
"#,
    );

    assert!(
        entities
            .iter()
            .any(|e| matches!(e, PostgresEntity::Enum(en) if en.name.as_ref() == "Status"))
    );
    assert!(
        entities
            .iter()
            .any(|e| matches!(e, PostgresEntity::View(v) if v.name.as_ref() == "thing_ids"))
    );
    assert!(
        entities
            .iter()
            .any(|e| matches!(e, PostgresEntity::Policy(p) if p.name.as_ref() == "things_policy"))
    );
    assert!(entities.iter().any(
        |e| matches!(e, PostgresEntity::UniqueConstraint(u) if u.name.as_ref() == "things_a_b_key")
    ));
    assert!(entities.iter().any(
        |e| matches!(e, PostgresEntity::CheckConstraint(c) if c.name.as_ref() == "things_check")
    ));
}

/// P10: SQLite generated / collate / check are no longer dropped.
#[test]
fn sqlite_generated_collate_check() {
    let entities = sqlite_entities(
        r#"
#[SQLiteTable]
pub struct People {
    #[column(primary)]
    pub id: i64,
    #[column(collate = NOCASE)]
    pub name: String,
    #[column(generated(virtual, "id * 2"))]
    pub double_id: i64,
    #[column(check = "id > 0")]
    pub checked: i64,
}
"#,
    );

    let column = |name: &str| {
        entities
            .iter()
            .find_map(|e| match e {
                SqliteEntity::Column(c) if c.name.as_ref() == name => Some(c),
                _ => None,
            })
            .expect("column")
    };
    assert_eq!(column("name").collate.as_deref(), Some("NOCASE"));
    let generated = column("double_id").generated.as_ref().expect("generated");
    assert_eq!(generated.expression.as_ref(), "(id * 2)");
    assert!(entities.iter().any(|e| matches!(
        e,
        SqliteEntity::CheckConstraint(c) if c.name.as_ref() == "people_checked_check"
    )));
}

/// P11: cfg-gated duplicates dedupe deterministically with a warning.
#[test]
fn cfg_duplicates_are_deterministic() {
    let code = r#"
#[cfg(feature = "uuid")]
#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub id: uuid::Uuid,
}

#[cfg(not(feature = "uuid"))]
#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub id: i64,
}
"#;
    let result = SchemaParser::parse(code);
    assert!(result.warnings.iter().any(|w| w.contains("duplicate")));
    let users = result.table("Users", Dialect::SQLite).expect("table");
    assert_eq!(users.field("id").expect("id").ty, "uuid::Uuid");
}

/// P12: `std::option::Option<T>` nullability and path-form dialect
/// attributes are recognized.
#[test]
fn qualified_paths_are_recognized() {
    let entities = sqlite_entities(
        r#"
#[drizzle::SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub id: i64,
    pub email: std::option::Option<String>,
}
"#,
    );
    let email = entities
        .iter()
        .find_map(|e| match e {
            SqliteEntity::Column(c) if c.name.as_ref() == "email" => Some(c),
            _ => None,
        })
        .expect("email column");
    assert!(!email.not_null);
}

/// P13: PostgreSQL deferrable FKs, array dimensions, and Vec<String> ->
/// text[] shape.
#[test]
fn postgres_deferrable_and_arrays() {
    let entities = postgres_entities(
        r#"
#[PostgresTable]
pub struct Users {
    #[column(primary)]
    pub id: i32,
    pub tags: Vec<String>,
}

#[PostgresTable]
pub struct Sessions {
    #[column(primary)]
    pub id: i32,
    #[column(references = Users::id, deferrable, initially_deferred)]
    pub user_id: i32,
}
"#,
    );

    let tags = entities
        .iter()
        .find_map(|e| match e {
            PostgresEntity::Column(c) if c.name.as_ref() == "tags" => Some(c),
            _ => None,
        })
        .expect("tags column");
    assert_eq!(tags.sql_type.as_ref(), "TEXT");
    assert_eq!(tags.dimensions, Some(1));

    let fk = entities
        .iter()
        .find_map(|e| match e {
            PostgresEntity::ForeignKey(fk) => Some(fk),
            _ => None,
        })
        .expect("fk");
    assert!(fk.deferrable && fk.initially_deferred);
}

/// P14: canonical constraint names for both dialects.
#[test]
fn canonical_constraint_names() {
    let sqlite = sqlite_entities(
        r#"
#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub id: i64,
    #[column(references = Users::id)]
    pub parent: Option<i64>,
}
"#,
    );
    assert!(
        sqlite
            .iter()
            .any(|e| matches!(e, SqliteEntity::PrimaryKey(pk) if pk.name.as_ref() == "users_pk"))
    );
    assert!(sqlite.iter().any(|e| matches!(
        e,
        SqliteEntity::ForeignKey(fk) if fk.name.as_ref() == "fk_users_parent_users_id_fk"
    )));

    let pg = postgres_entities(
        r#"
#[PostgresTable]
pub struct Users {
    #[column(primary)]
    pub id: i32,
    #[column(unique)]
    pub email: String,
    #[column(references = Users::id)]
    pub parent: Option<i32>,
}
"#,
    );
    assert!(
        pg.iter().any(
            |e| matches!(e, PostgresEntity::PrimaryKey(pk) if pk.name.as_ref() == "users_pkey")
        )
    );
    assert!(pg.iter().any(
        |e| matches!(e, PostgresEntity::UniqueConstraint(u) if u.name.as_ref() == "users_email_key")
    ));
    assert!(pg.iter().any(
        |e| matches!(e, PostgresEntity::ForeignKey(fk) if fk.name.as_ref() == "users_parent_fkey")
    ));
}

/// P15: the casing config cannot produce names the runtime never uses.
#[test]
fn casing_config_is_inert_for_names() {
    let code = r#"
#[SQLiteTable]
pub struct UserAccounts {
    #[column(primary)]
    pub id: i64,
    pub displayName: String,
}
"#;
    let result = SchemaParser::parse(code);
    let snapshot = Snapshot::from_parse_result(
        &result,
        Dialect::SQLite,
        Some(drizzle_types::Casing::CamelCase),
    );
    let Snapshot::Sqlite(snap) = snapshot else {
        panic!("expected sqlite snapshot")
    };
    assert!(
        snap.ddl
            .iter()
            .any(|e| matches!(e, SqliteEntity::Table(t) if t.name.as_ref() == "user_accounts"))
    );
    assert!(
        snap.ddl
            .iter()
            .any(|e| matches!(e, SqliteEntity::Column(c) if c.name.as_ref() == "display_name"))
    );
}

/// P16 / m10: parse failures are loud, not silent.
#[test]
fn parse_failures_are_loud() {
    let result = SchemaParser::parse("#[SQLiteTable]\nstruct Broken {");
    assert!(result.tables.is_empty());
    assert!(!result.errors.is_empty());

    // A macro-invalid attribute records an error but still emits the table.
    let result = SchemaParser::parse(
        r#"
#[SQLiteTable(frobnicate = 3)]
pub struct Users {
    #[column(primary)]
    pub id: i64,
}
"#,
    );
    assert!(!result.errors.is_empty());
    assert!(result.table("Users", Dialect::SQLite).is_some());
}

/// Schema-struct membership scopes tables (and their indexes) exactly like
/// the runtime `Schema::to_snapshot()` does.
#[test]
fn schema_membership_scopes_tables() {
    let entities = sqlite_entities(
        r#"
#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub id: i64,
    pub email: String,
}

#[SQLiteTable]
pub struct Orphan {
    #[column(primary)]
    pub id: i64,
}

#[SQLiteIndex(unique)]
pub struct IdxUsersEmail(Users::email);

#[SQLiteIndex]
pub struct IdxOrphan(Orphan::id);

#[derive(SQLiteSchema)]
pub struct Schema {
    pub users: Users,
    pub idx_users_email: IdxUsersEmail,
}
"#,
    );

    let tables: Vec<&str> = entities
        .iter()
        .filter_map(|e| match e {
            SqliteEntity::Table(t) => Some(t.name.as_ref()),
            _ => None,
        })
        .collect();
    assert_eq!(tables, vec!["users"]);

    let indexes: Vec<&str> = entities
        .iter()
        .filter_map(|e| match e {
            SqliteEntity::Index(i) => Some(i.name.as_ref()),
            _ => None,
        })
        .collect();
    assert_eq!(indexes, vec!["idx_users_email"]);
}

/// Multi-file concatenation (the build.rs / CLI input shape) resolves
/// cross-file FKs through the name maps.
#[test]
fn multi_file_concatenation() {
    let users = "//! users schema\n#[SQLiteTable(name = \"users_tbl\")]\npub struct Users {\n    #[column(primary)]\n    pub id: i64,\n}\n";
    let posts = "//! posts schema\n#[SQLiteTable]\npub struct Posts {\n    #[column(primary)]\n    pub id: i64,\n    #[column(references = Users::id)]\n    pub author_id: i64,\n}\n";
    let combined = format!("{users}\n{posts}");

    let entities = sqlite_entities(&combined);
    let fk = entities
        .iter()
        .find_map(|e| match e {
            SqliteEntity::ForeignKey(fk) => Some(fk),
            _ => None,
        })
        .expect("fk");
    // Target table AND the FK name resolve through the referenced table's
    // explicit name (macro behavior: names derive from `TABLE_NAME` /
    // column `NAME` consts, not re-derived struct idents).
    assert_eq!(fk.table_to.as_ref(), "users_tbl");
    assert_eq!(fk.name.as_ref(), "fk_posts_author_id_users_tbl_id_fk");
}
