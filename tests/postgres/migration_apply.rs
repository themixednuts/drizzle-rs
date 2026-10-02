//! Runs generated PostgreSQL migrations against a real server.
//!
//! Each test works in its own freshly created schema (dropped again at the
//! end) and needs the local test database (`DATABASE_URL`, default
//! `host=localhost user=postgres password=postgres dbname=drizzle_test`,
//! see `docker-compose.yml`). The `public` schema is never touched.
//!
//! Two kinds of checks:
//!
//! - **apply**: build `prev` from nothing, migrate `prev` -> `cur`, and the
//!   server must accept every statement;
//! - **push round trip**: create `desired`, introspect it back, and pushing
//!   the same schema again must produce no statements.

use drizzle_migrations::postgres::PostgresSnapshot;
use drizzle_migrations::postgres::collection::PostgresDDL;
use drizzle_migrations::postgres::ddl::{
    CheckConstraint, Column, Enum, ForeignKey, Generated, GeneratedType, Identity, Index,
    IndexColumn, Opclass, Policy, PrimaryKey, Schema, Table, UniqueConstraint, View,
};
use drizzle_migrations::postgres::diff::compute_migration;
use drizzle_migrations::postgres::introspect::{
    RawCheckInfo, RawColumnInfo, RawEnumInfo, RawForeignKeyInfo, RawIndexInfo, RawIntrospection,
    RawPolicyInfo, RawPrimaryKeyInfo, RawSequenceInfo, RawTableInfo, RawUniqueInfo, RawViewInfo,
    action_code_to_string, assemble_ddl, parse_index_columns, queries,
};
use drizzle_migrations::schema::Snapshot;
use postgres::{Client, NoTls};
use std::borrow::Cow;
use std::sync::atomic::{AtomicUsize, Ordering};

fn database_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "host=localhost user=postgres password=postgres dbname=drizzle_test".to_string()
    })
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A connection whose `search_path` is a private schema, dropped on drop.
struct TestSchema {
    client: Client,
    name: String,
}

impl TestSchema {
    fn new(label: &str) -> Self {
        let mut client = Client::connect(&database_url(), NoTls)
            .expect("connect to the PostgreSQL test database (see docker-compose.yml)");
        let name = format!(
            "drizzle_mig_{label}_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        client
            .batch_execute(&format!(
                "DROP SCHEMA IF EXISTS \"{name}\" CASCADE; CREATE SCHEMA \"{name}\"; SET search_path TO \"{name}\";"
            ))
            .expect("create test schema");
        Self { client, name }
    }

    /// Runs each statement on its own, the way drivers do.
    fn run(&mut self, statements: &[String]) {
        for statement in statements {
            if let Err(error) = self.client.batch_execute(statement) {
                panic!(
                    "PostgreSQL rejected:\n{statement}\n\nerror: {}\n\nall statements:\n{}",
                    error
                        .as_db_error()
                        .map_or_else(|| error.to_string(), ToString::to_string),
                    statements.join("\n")
                );
            }
        }
    }

    /// Builds `prev` from nothing, then migrates it to `cur`. Returns the
    /// migration's statements.
    fn apply(&mut self, prev: &PostgresDDL, cur: &PostgresDDL, data: &str) -> Vec<String> {
        let setup = compute_migration(&PostgresDDL::new(), prev).sql_statements;
        self.run(&setup);
        if !data.is_empty() {
            self.client.batch_execute(data).expect("seed data");
        }
        let migration = compute_migration(prev, cur).sql_statements;
        self.run(&migration);
        migration
    }

    /// Introspects this test's schema (scoped like `push` scopes it).
    fn introspect(&mut self) -> PostgresDDL {
        let schemas = vec![self.name.clone()];
        let client = &mut self.client;
        let in_scope = |schema: &String| schema == &schemas[0];

        let tables = client
            .query(queries::TABLES_QUERY, &[])
            .expect("tables")
            .into_iter()
            .map(|row| RawTableInfo {
                schema: row.get(0),
                name: row.get(1),
                is_rls_enabled: row.get(2),
                is_unlogged: row.get(3),
                is_temporary: row.get(4),
                tablespace: row.get(5),
                comment: row.get(6),
            })
            .filter(|t| in_scope(&t.schema))
            .collect();
        let columns = client
            .query(queries::COLUMNS_QUERY, &[])
            .expect("columns")
            .into_iter()
            .map(|row| RawColumnInfo {
                schema: row.get(0),
                table: row.get(1),
                name: row.get(2),
                column_type: row.get(3),
                type_schema: row.get(4),
                not_null: row.get(5),
                default_value: row.get(6),
                is_identity: row.get(7),
                identity_type: row.get(8),
                is_generated: row.get(9),
                generated_expression: row.get(10),
                generated_stored: row.get(11),
                dimensions: row.get(12),
                comment: row.get(13),
                ordinal_position: row.get(14),
            })
            .filter(|c| in_scope(&c.schema))
            .collect();
        let enums = client
            .query(queries::ENUMS_QUERY, &[])
            .expect("enums")
            .into_iter()
            .map(|row| RawEnumInfo {
                schema: row.get(0),
                name: row.get(1),
                values: row.get(2),
            })
            .filter(|e| in_scope(&e.schema))
            .collect();
        let sequences = client
            .query(queries::SEQUENCES_QUERY, &[])
            .expect("sequences")
            .into_iter()
            .map(|row| RawSequenceInfo {
                schema: row.get(0),
                name: row.get(1),
                data_type: row.get(2),
                start_value: row.get(3),
                min_value: row.get(4),
                max_value: row.get(5),
                increment: row.get(6),
                cycle: row.get(7),
                cache_value: row.get(8),
                owned_by: row.get(9),
            })
            .filter(|s| in_scope(&s.schema))
            .collect();
        let views = client
            .query(queries::VIEWS_QUERY, &[&Some(&schemas)])
            .expect("views")
            .into_iter()
            .map(|row| RawViewInfo {
                schema: row.get(0),
                name: row.get(1),
                definition: row.get(2),
                is_materialized: row.get(3),
            })
            .collect();
        let indexes = client
            .query(queries::INDEXES_QUERY_FILTERED, &[&schemas])
            .expect("indexes")
            .into_iter()
            .map(|row| RawIndexInfo {
                schema: row.get(0),
                table: row.get(1),
                name: row.get(2),
                is_unique: row.get(3),
                is_primary: row.get(4),
                method: row.get(5),
                columns: parse_index_columns(row.get(6)),
                where_clause: row.get(7),
                concurrent: false,
            })
            .collect();
        let foreign_keys = client
            .query(queries::FOREIGN_KEYS_QUERY, &[])
            .expect("fks")
            .into_iter()
            .map(|row| RawForeignKeyInfo {
                schema: row.get(0),
                table: row.get(1),
                name: row.get(2),
                columns: row.get(3),
                schema_to: row.get(4),
                table_to: row.get(5),
                columns_to: row.get(6),
                on_update: action_code_to_string(&row.get::<_, String>(7)),
                on_delete: action_code_to_string(&row.get::<_, String>(8)),
                deferrable: row.get(9),
                initially_deferred: row.get(10),
            })
            .filter(|f| in_scope(&f.schema))
            .collect();
        let primary_keys = client
            .query(queries::PRIMARY_KEYS_QUERY, &[])
            .expect("pks")
            .into_iter()
            .map(|row| RawPrimaryKeyInfo {
                schema: row.get(0),
                table: row.get(1),
                name: row.get(2),
                columns: row.get(3),
            })
            .filter(|p| in_scope(&p.schema))
            .collect();
        let unique_constraints = client
            .query(queries::UNIQUES_QUERY, &[])
            .expect("uniques")
            .into_iter()
            .map(|row| RawUniqueInfo {
                schema: row.get(0),
                table: row.get(1),
                name: row.get(2),
                columns: row.get(3),
                nulls_not_distinct: row.get(4),
                deferrable: row.get(5),
                initially_deferred: row.get(6),
            })
            .filter(|u| in_scope(&u.schema))
            .collect();
        let check_constraints = client
            .query(queries::CHECKS_QUERY_FILTERED, &[&schemas])
            .expect("checks")
            .into_iter()
            .map(|row| RawCheckInfo {
                schema: row.get(0),
                table: row.get(1),
                name: row.get(2),
                expression: row.get(3),
            })
            .collect();
        let policies = client
            .query(queries::POLICIES_QUERY, &[])
            .expect("policies")
            .into_iter()
            .map(|row| RawPolicyInfo {
                schema: row.get(0),
                table: row.get(1),
                name: row.get(2),
                as_clause: row.get(3),
                for_clause: row.get(4),
                to: row.get(5),
                using: row.get(6),
                with_check: row.get(7),
            })
            .filter(|p| in_scope(&p.schema))
            .collect();

        assemble_ddl(RawIntrospection {
            schemas: vec![Schema::new(self.name.clone())],
            tables,
            columns,
            enums,
            sequences,
            views,
            indexes,
            foreign_keys,
            primary_keys,
            unique_constraints,
            check_constraints,
            roles: Vec::new(),
            policies,
        })
    }

    /// Creates `desired`, then returns what pushing it again would run.
    fn push_twice(&mut self, desired: &PostgresDDL) -> Vec<String> {
        // The test schema already exists.
        let mut create = desired.clone();
        create.schemas = PostgresDDL::new().schemas;
        let create = compute_migration(&PostgresDDL::new(), &create).sql_statements;
        self.run(&create);

        let snapshot = |ddl: &PostgresDDL| {
            let mut snapshot = PostgresSnapshot::new();
            for entity in ddl.to_entities() {
                snapshot.add_entity(entity);
            }
            snapshot
        };
        let desired = snapshot(desired);
        let live = snapshot(&self.introspect()).prepare_for_push(&desired);
        drizzle_migrations::diff(&Snapshot::Postgres(live), &Snapshot::Postgres(desired))
            .expect("diff")
            .statements
    }
}

impl Drop for TestSchema {
    fn drop(&mut self) {
        let _ = self
            .client
            .batch_execute(&format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE;", self.name));
    }
}

// =============================================================================
// DDL builders (`public` objects land in the test schema via search_path)
// =============================================================================

fn table(ddl: &mut PostgresDDL, schema: &str, name: &str) {
    ddl.tables
        .push(Table::new(schema.to_string(), name.to_string()));
}

fn column(ddl: &mut PostgresDDL, schema: &str, table: &str, name: &str, ty: &str) {
    ddl.columns.push(Column::new(
        schema.to_string(),
        table.to_string(),
        name.to_string(),
        ty.to_string(),
    ));
}

fn col<'a>(ddl: &'a mut PostgresDDL, table: &str, name: &str) -> &'a mut Column {
    ddl.columns
        .list_mut()
        .iter_mut()
        .find(|c| c.table == table && c.name == name)
        .expect("column")
}

fn pk(ddl: &mut PostgresDDL, schema: &str, table: &str, columns: &[&str]) {
    let mut pk = PrimaryKey::from_strings(
        schema.to_string(),
        table.to_string(),
        format!("{table}_pkey"),
        columns.iter().map(ToString::to_string).collect(),
    );
    pk.name_explicit = false;
    ddl.pks.push(pk);
}

fn fk(
    ddl: &mut PostgresDDL,
    schema: &str,
    table: &str,
    name: &str,
    columns: &[&str],
    to: &str,
    to_columns: &[&str],
) {
    ddl.fks.push(ForeignKey::from_strings(
        schema.to_string(),
        table.to_string(),
        name.to_string(),
        columns.iter().map(ToString::to_string).collect(),
        schema.to_string(),
        to.to_string(),
        to_columns.iter().map(ToString::to_string).collect(),
    ));
}

fn unique(ddl: &mut PostgresDDL, schema: &str, table: &str, name: &str, columns: &[&str]) {
    ddl.uniques.push(UniqueConstraint::from_strings(
        schema.to_string(),
        table.to_string(),
        name.to_string(),
        columns.iter().map(ToString::to_string).collect(),
    ));
}

fn enum_type(ddl: &mut PostgresDDL, schema: &str, name: &str, values: &[&str]) {
    ddl.enums.push(Enum::from_strings(
        schema.to_string(),
        name.to_string(),
        values.iter().map(ToString::to_string).collect(),
    ));
}

fn enum_column(ddl: &mut PostgresDDL, table: &str, name: &str, enum_name: &str) {
    column(ddl, "public", table, name, enum_name);
    col(ddl, table, name).type_schema = Some(Cow::Borrowed("public"));
}

fn users(ddl: &mut PostgresDDL) {
    table(ddl, "public", "users");
    column(ddl, "public", "users", "id", "integer");
    col(ddl, "users", "id").not_null = true;
    pk(ddl, "public", "users", &["id"]);
}

fn posts(ddl: &mut PostgresDDL, references: &str) {
    table(ddl, "public", "posts");
    column(ddl, "public", "posts", "id", "integer");
    col(ddl, "posts", "id").not_null = true;
    column(ddl, "public", "posts", "user_id", "integer");
    pk(ddl, "public", "posts", &["id"]);
    fk(
        ddl,
        "public",
        "posts",
        "posts_user_id_fkey",
        &["user_id"],
        references,
        &["id"],
    );
}

fn stored(expression: &'static str) -> Option<Generated> {
    Some(Generated {
        expression: Cow::Borrowed(expression),
        gen_type: GeneratedType::Stored,
    })
}

// =============================================================================
// apply: the server accepts the migration
// =============================================================================

#[test]
fn first_migration_and_dropping_the_last_public_table_leave_public_alone() {
    let mut db = TestSchema::new("public_schema");
    let mut prev = PostgresDDL::new();
    prev.schemas.push(Schema::new("public"));
    users(&mut prev);
    let statements = db.apply(&prev, &PostgresDDL::new(), "");
    assert_eq!(statements, ["DROP TABLE \"users\";"]);
}

#[test]
fn drop_fk_linked_tables() {
    let mut db = TestSchema::new("drop_linked");
    let mut prev = PostgresDDL::new();
    users(&mut prev);
    posts(&mut prev, "users");
    db.apply(&prev, &PostgresDDL::new(), "INSERT INTO users VALUES (1);");
}

#[test]
fn drop_table_and_the_unique_its_fk_references() {
    let mut db = TestSchema::new("drop_unique_ref");
    let mut prev = PostgresDDL::new();
    users(&mut prev);
    column(&mut prev, "public", "users", "email", "text");
    unique(&mut prev, "public", "users", "users_email_key", &["email"]);
    table(&mut prev, "public", "invites");
    column(&mut prev, "public", "invites", "email", "text");
    fk(
        &mut prev,
        "public",
        "invites",
        "invites_email_fkey",
        &["email"],
        "users",
        &["email"],
    );
    let mut cur = PostgresDDL::new();
    users(&mut cur);
    db.apply(&prev, &cur, "");
}

#[test]
fn table_rename_keeps_the_primary_key_an_fk_depends_on() {
    let mut db = TestSchema::new("rename_pk");
    let mut prev = PostgresDDL::new();
    users(&mut prev);
    posts(&mut prev, "users");
    let mut cur = PostgresDDL::new();
    table(&mut cur, "public", "accounts");
    column(&mut cur, "public", "accounts", "id", "integer");
    col(&mut cur, "accounts", "id").not_null = true;
    pk(&mut cur, "public", "accounts", &["id"]);
    posts(&mut cur, "accounts");
    let statements = db.apply(&prev, &cur, "INSERT INTO users VALUES (1);");
    // Like drizzle-kit, the implicitly named key keeps the name it has.
    assert_eq!(statements, ["ALTER TABLE \"users\" RENAME TO \"accounts\";"]);
    let pk_name: String = db
        .client
        .query_one(
            "SELECT c.conname::text FROM pg_constraint c \
             JOIN pg_class t ON t.oid = c.conrelid \
             JOIN pg_namespace n ON n.oid = t.relnamespace \
             WHERE n.nspname = $1 AND t.relname = 'accounts' AND c.contype = 'p'",
            &[&db.name],
        )
        .expect("primary key")
        .get(0);
    assert_eq!(pk_name, "users_pkey");

    // Column rename with a default-named unique.
    let mut db = TestSchema::new("rename_unique");
    let mut prev = PostgresDDL::new();
    users(&mut prev);
    column(&mut prev, "public", "users", "email", "text");
    let mut unique_email = UniqueConstraint::from_strings(
        "public".into(),
        "users".into(),
        "users_email_key".into(),
        vec!["email".into()],
    );
    unique_email.name_explicit = false;
    prev.uniques.push(unique_email);
    let mut cur = PostgresDDL::new();
    users(&mut cur);
    column(&mut cur, "public", "users", "mail", "text");
    let mut unique_mail = UniqueConstraint::from_strings(
        "public".into(),
        "users".into(),
        "users_mail_key".into(),
        vec!["mail".into()],
    );
    unique_mail.name_explicit = false;
    cur.uniques.push(unique_mail);
    db.apply(&prev, &cur, "INSERT INTO users VALUES (1, 'a@b');");
}

#[test]
fn view_definition_change_with_column_drop_and_type_change() {
    let mut db = TestSchema::new("view_alter");
    let mut prev = PostgresDDL::new();
    users(&mut prev);
    column(&mut prev, "public", "users", "legacy", "text");
    column(&mut prev, "public", "users", "age", "integer");
    let mut view = View::new("public", "v_users");
    view.definition = Some(Cow::Borrowed("select id, legacy, age from users"));
    prev.views.push(view);
    let mut cur = PostgresDDL::new();
    users(&mut cur);
    column(&mut cur, "public", "users", "age", "bigint");
    let mut view = View::new("public", "v_users");
    view.definition = Some(Cow::Borrowed("select id, age from users"));
    cur.views.push(view);
    db.apply(&prev, &cur, "INSERT INTO users VALUES (1, 'x', 2);");
}

#[test]
fn enum_changes() {
    // enum -> another enum, with a default
    let mut db = TestSchema::new("enum_to_enum");
    let mut prev = PostgresDDL::new();
    enum_type(&mut prev, "public", "status_a", &["x", "y"]);
    enum_type(&mut prev, "public", "status_b", &["x", "y"]);
    table(&mut prev, "public", "items");
    enum_column(&mut prev, "items", "status", "status_a");
    col(&mut prev, "items", "status").default = Some(Cow::Borrowed("'x'"));
    let mut cur = prev.clone();
    col(&mut cur, "items", "status").sql_type = Cow::Borrowed("status_b");
    db.apply(&prev, &cur, "INSERT INTO items(status) VALUES ('y');");

    // value removed from an enum used by an array column and a defaulted
    // mixed-case column
    let mut db = TestSchema::new("enum_recreate");
    let mut prev = PostgresDDL::new();
    enum_type(&mut prev, "public", "Mood", &["a", "b", "c"]);
    table(&mut prev, "public", "people");
    enum_column(&mut prev, "people", "moods", "Mood");
    col(&mut prev, "people", "moods").dimensions = Some(1);
    enum_column(&mut prev, "people", "m", "Mood");
    col(&mut prev, "people", "m").default = Some(Cow::Borrowed("'a'"));
    let mut cur = prev.clone();
    cur.enums.list_mut()[0].values = Cow::Owned(vec![Cow::Borrowed("a"), Cow::Borrowed("b")]);
    db.apply(
        &prev,
        &cur,
        "INSERT INTO people(moods, m) VALUES ('{a,b}', 'b');",
    );

    // value removed while a column moves off the enum, another is dropped
    let mut db = TestSchema::new("enum_leave");
    let mut prev = PostgresDDL::new();
    enum_type(&mut prev, "public", "mood", &["a", "b", "c"]);
    table(&mut prev, "public", "people");
    enum_column(&mut prev, "people", "m", "mood");
    col(&mut prev, "people", "m").default = Some(Cow::Borrowed("'a'"));
    enum_column(&mut prev, "people", "gone", "mood");
    let mut cur = PostgresDDL::new();
    enum_type(&mut cur, "public", "mood", &["a", "b"]);
    table(&mut cur, "public", "people");
    column(&mut cur, "public", "people", "m", "text");
    col(&mut cur, "people", "m").default = Some(Cow::Borrowed("'z'"));
    db.apply(&prev, &cur, "INSERT INTO people(m) VALUES ('c');");

    // value removed while new tables/columns start using the enum and a
    // table using it is dropped
    let mut db = TestSchema::new("enum_new_users");
    let mut prev = PostgresDDL::new();
    enum_type(&mut prev, "public", "mood", &["a", "b", "c"]);
    table(&mut prev, "public", "people");
    enum_column(&mut prev, "people", "m", "mood");
    table(&mut prev, "public", "old_people");
    enum_column(&mut prev, "old_people", "m", "mood");
    let mut cur = PostgresDDL::new();
    enum_type(&mut cur, "public", "mood", &["a", "b"]);
    table(&mut cur, "public", "people");
    enum_column(&mut cur, "people", "m", "mood");
    enum_column(&mut cur, "people", "m2", "mood");
    col(&mut cur, "people", "m2").default = Some(Cow::Borrowed("'b'"));
    table(&mut cur, "public", "fresh");
    // (an extra column, so `old_people` -> `fresh` is not a rename)
    column(&mut cur, "public", "fresh", "id", "integer");
    enum_column(&mut cur, "fresh", "m", "mood");
    db.apply(
        &prev,
        &cur,
        "INSERT INTO people(m) VALUES ('a'); INSERT INTO old_people(m) VALUES ('c');",
    );
}

#[test]
fn serial_transitions() {
    for (from, to) in [
        ("integer", "serial"),
        ("serial", "integer"),
        ("serial", "bigserial"),
        ("bigserial", "serial"),
        ("bigint", "serial"),
    ] {
        let mut db = TestSchema::new("serial");
        let mut prev = PostgresDDL::new();
        table(&mut prev, "public", "s");
        column(&mut prev, "public", "s", "n", from);
        col(&mut prev, "s", "n").not_null = true;
        let mut cur = prev.clone();
        col(&mut cur, "s", "n").sql_type = Cow::Owned(to.to_string());
        db.apply(&prev, &cur, "INSERT INTO s VALUES (7);");
        if to.ends_with("serial") {
            db.run(&["INSERT INTO s DEFAULT VALUES;".to_string()]);
        } else {
            let has_default: bool = db
                .client
                .query_one(
                    "SELECT column_default IS NOT NULL FROM information_schema.columns \
                     WHERE table_schema = current_schema() AND table_name = 's'",
                    &[],
                )
                .expect("default")
                .get(0);
            assert!(!has_default, "{from} -> {to} must drop the nextval default");
        }
    }
}

#[test]
fn new_table_fk_to_unique_added_in_the_same_migration() {
    let mut db = TestSchema::new("fk_new_unique");
    let mut prev = PostgresDDL::new();
    users(&mut prev);
    column(&mut prev, "public", "users", "email", "text");
    let mut cur = prev.clone();
    unique(&mut cur, "public", "users", "users_email_key", &["email"]);
    table(&mut cur, "public", "invites");
    column(&mut cur, "public", "invites", "email", "text");
    fk(
        &mut cur,
        "public",
        "invites",
        "invites_email_fkey",
        &["email"],
        "users",
        &["email"],
    );
    db.apply(&prev, &cur, "");
}

#[test]
fn generated_rls_types_collation_policies_and_comments() {
    let mut db = TestSchema::new("misc");
    let mut prev = PostgresDDL::new();
    users(&mut prev);
    column(&mut prev, "public", "users", "a", "integer");
    column(&mut prev, "public", "users", "b", "integer");
    col(&mut prev, "users", "b").generated = stored("a + 1");
    column(&mut prev, "public", "users", "c", "integer");
    prev.indexes.push(Index::new(
        "public",
        "users",
        "users_c_idx",
        vec![IndexColumn::new("c")],
    ));
    unique(&mut prev, "public", "users", "users_c_key", &["c"]);
    table(&mut prev, "public", "refs");
    column(&mut prev, "public", "refs", "c", "integer");
    fk(
        &mut prev,
        "public",
        "refs",
        "refs_c_fkey",
        &["c"],
        "users",
        &["c"],
    );
    column(&mut prev, "public", "users", "at", "timestamp(3)");
    column(&mut prev, "public", "users", "name", "text");
    enum_type(&mut prev, "public", "time_unit", &["s"]);
    enum_type(&mut prev, "public", "time_zone", &["s"]);
    enum_column(&mut prev, "users", "unit", "time_unit");

    let mut cur = prev.clone();
    col(&mut cur, "users", "b").generated = stored("a + 2");
    col(&mut cur, "users", "c").generated = stored("a * 10");
    col(&mut cur, "users", "at").sql_type = Cow::Borrowed("timestamp(0)");
    col(&mut cur, "users", "name").collate = Some(Cow::Borrowed("C"));
    col(&mut cur, "users", "name").comment = Some(Cow::Borrowed("first;\nsecond 'quoted'"));
    col(&mut cur, "users", "unit").sql_type = Cow::Borrowed("time_zone");
    let mut policy = Policy::new("public", "users", "own_rows");
    policy.to = Some(vec![Cow::Borrowed("current_user")]);
    policy.using = Some(Cow::Borrowed("true"));
    cur.policies.push(policy);

    db.apply(
        &prev,
        &cur,
        "INSERT INTO users(id, a, c, at, name) VALUES (1, 1, 5, now(), 'x');",
    );

    assert!(db.check("SELECT relrowsecurity FROM pg_class WHERE oid = 'users'::regclass"));
    assert!(db.check(
        "SELECT pg_get_expr(adbin, adrelid) LIKE '%a + 2%' FROM pg_attrdef \
         WHERE adrelid = 'users'::regclass AND adnum = (SELECT attnum FROM pg_attribute \
         WHERE attrelid = 'users'::regclass AND attname = 'b')"
    ));
    assert!(db.check(
        "SELECT count(*) = 2 FROM pg_constraint WHERE conname IN ('users_c_key', 'refs_c_fkey')"
    ));
    assert!(db.check(
        "SELECT count(*) = 1 FROM pg_indexes WHERE schemaname = current_schema() AND indexname = 'users_c_idx'"
    ));
    assert!(db.check(
        "SELECT format_type(atttypid, atttypmod) = 'timestamp(0) without time zone' \
         FROM pg_attribute WHERE attrelid = 'users'::regclass AND attname = 'at'"
    ));
    assert!(db.check(
        "SELECT col_description('users'::regclass, (SELECT attnum FROM pg_attribute \
         WHERE attrelid = 'users'::regclass AND attname = 'name')) = E'first;\\nsecond ''quoted'''"
    ));

    // Removing the only policy turns row-level security off again.
    db.apply_from(&cur, &prev);
    assert!(!db.check("SELECT relrowsecurity FROM pg_class WHERE oid = 'users'::regclass"));
}

impl TestSchema {
    fn check(&mut self, sql: &str) -> bool {
        self.client
            .query_one(sql, &[])
            .expect("query")
            .get::<_, bool>(0)
    }

    /// Migrates the objects already created for `prev` to `cur`.
    fn apply_from(&mut self, prev: &PostgresDDL, cur: &PostgresDDL) {
        let migration = compute_migration(prev, cur).sql_statements;
        self.run(&migration);
    }
}

// =============================================================================
// push round trip: introspecting what was created gives the same schema
// =============================================================================

#[test]
fn second_push_of_an_unchanged_schema_is_empty() {
    let mut db = TestSchema::new("push");
    let s: &'static str = Box::leak(db.name.clone().into_boxed_str());
    let mut desired = PostgresDDL::new();
    desired.schemas.push(Schema::new(s));
    enum_type(&mut desired, s, "Status", &["active", "archived"]);

    table(&mut desired, s, "accounts");
    column(&mut desired, s, "accounts", "id", "INTEGER");
    col(&mut desired, "accounts", "id").not_null = true;
    let mut identity = Identity::by_default("accounts_id_seq");
    identity.cache = Some(5);
    col(&mut desired, "accounts", "id").identity = Some(identity);
    pk(&mut desired, s, "accounts", &["id"]);
    column(&mut desired, s, "accounts", "email", "TEXT");
    col(&mut desired, "accounts", "email").collate = Some(Cow::Borrowed("C"));
    unique(
        &mut desired,
        s,
        "accounts",
        "accounts_email_key",
        &["email"],
    );
    column(&mut desired, s, "accounts", "score", "DOUBLE PRECISION");
    column(&mut desired, s, "accounts", "ratio", "REAL");
    column(&mut desired, s, "accounts", "balance", "INTEGER");
    col(&mut desired, "accounts", "balance").default = Some(Cow::Borrowed("-1"));
    column(&mut desired, s, "accounts", "active", "BOOLEAN");
    col(&mut desired, "accounts", "active").default = Some(Cow::Borrowed("TRUE"));
    column(&mut desired, s, "accounts", "tags", "varchar(10)");
    col(&mut desired, "accounts", "tags").dimensions = Some(1);
    column(&mut desired, s, "accounts", "amounts", "numeric(10,2)");
    col(&mut desired, "accounts", "amounts").dimensions = Some(1);
    column(
        &mut desired,
        s,
        "accounts",
        "seen_at",
        "timestamp(3) with time zone",
    );
    column(&mut desired, s, "accounts", "status", "Status");
    col(&mut desired, "accounts", "status").type_schema = Some(Cow::Owned(s.to_string()));
    col(&mut desired, "accounts", "status").default = Some(Cow::Borrowed("'active'"));
    column(&mut desired, s, "accounts", "total", "integer");
    col(&mut desired, "accounts", "total").generated = stored("balance * 2");
    desired.checks.push(CheckConstraint::new(
        s,
        "accounts",
        "accounts_status_check",
        "email <> 'bad' AND balance > -100",
    ));
    let mut by_score = Index::new(
        s,
        "accounts",
        "accounts_score_idx",
        vec![
            IndexColumn::new("score").desc().nulls_last(),
            IndexColumn::new("email").with_opclass(Opclass::new("text_pattern_ops")),
        ],
    );
    by_score.where_clause = Some(Cow::Borrowed("score IS NOT NULL"));
    desired.indexes.push(by_score);
    let mut concurrent = Index::new(
        s,
        "accounts",
        "accounts_balance_idx",
        vec![IndexColumn::new("balance")],
    );
    concurrent.name_explicit = false;
    concurrent.concurrently = true;
    desired.indexes.push(concurrent);

    table(&mut desired, s, "notes");
    column(&mut desired, s, "notes", "id", "serial");
    col(&mut desired, "notes", "id").not_null = true;
    pk(&mut desired, s, "notes", &["id"]);
    column(&mut desired, s, "notes", "account_id", "integer");
    fk(
        &mut desired,
        s,
        "notes",
        "notes_account_id_fkey",
        &["account_id"],
        "accounts",
        &["id"],
    );
    let mut policy = Policy::new(s, "notes", "notes_owner");
    policy.using = Some(Cow::Borrowed("account_id = 1"));
    desired.policies.push(policy);
    let mut view = View::new(s, "active_accounts");
    view.definition = Some(Cow::Owned(format!(
        "select id, email from \"{s}\".accounts where status = 'active'"
    )));
    desired.views.push(view);

    let statements = db.push_twice(&desired);
    assert!(
        statements.is_empty(),
        "second push must not change anything: {statements:#?}"
    );
}
