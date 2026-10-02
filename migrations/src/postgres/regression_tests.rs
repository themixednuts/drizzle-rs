//! Regression tests for PostgreSQL migration generation: statement order,
//! column alters and other cases that once produced SQL PostgreSQL rejects.
//! `migrations/tests/postgres_migration_apply.rs` runs the same kinds of
//! migrations against a real server.

use super::collection::PostgresDDL;
use super::ddl::{
    Column, Enum, ForeignKey, Generated, GeneratedType, Index, IndexColumn, Policy, PrimaryKey,
    Table, UniqueConstraint, View,
};
use super::diff::compute_migration;
use std::borrow::Cow;

fn table(ddl: &mut PostgresDDL, name: &str) {
    ddl.tables.push(Table::new("public", name.to_string()));
}

fn column(ddl: &mut PostgresDDL, table: &str, name: &str, sql_type: &str) -> usize {
    ddl.columns.push(Column::new(
        "public",
        table.to_string(),
        name.to_string(),
        sql_type.to_string(),
    ));
    ddl.columns.list().len() - 1
}

fn col<'a>(ddl: &'a mut PostgresDDL, table: &str, name: &str) -> &'a mut Column {
    ddl.columns
        .list_mut()
        .iter_mut()
        .find(|c| c.table == table && c.name == name)
        .expect("column")
}

fn pk(ddl: &mut PostgresDDL, table: &str, columns: &[&str]) {
    let mut pk = PrimaryKey::from_strings(
        "public".to_string(),
        table.to_string(),
        format!("{table}_pkey"),
        columns.iter().map(ToString::to_string).collect(),
    );
    pk.name_explicit = false;
    ddl.pks.push(pk);
}

fn fk(
    ddl: &mut PostgresDDL,
    table: &str,
    name: &str,
    columns: &[&str],
    to: &str,
    to_cols: &[&str],
) {
    ddl.fks.push(ForeignKey::from_strings(
        "public".to_string(),
        table.to_string(),
        name.to_string(),
        columns.iter().map(ToString::to_string).collect(),
        "public".to_string(),
        to.to_string(),
        to_cols.iter().map(ToString::to_string).collect(),
    ));
}

fn unique(ddl: &mut PostgresDDL, table: &str, name: &str, columns: &[&str]) {
    ddl.uniques.push(UniqueConstraint::from_strings(
        "public".to_string(),
        table.to_string(),
        name.to_string(),
        columns.iter().map(ToString::to_string).collect(),
    ));
}

fn enum_type(ddl: &mut PostgresDDL, name: &str, values: &[&str]) {
    ddl.enums.push(Enum::from_strings(
        "public".to_string(),
        name.to_string(),
        values.iter().map(ToString::to_string).collect(),
    ));
}

fn enum_column(ddl: &mut PostgresDDL, table: &str, name: &str, enum_name: &str) {
    column(ddl, table, name, enum_name);
    col(ddl, table, name).type_schema = Some(Cow::Borrowed("public"));
}

fn users(ddl: &mut PostgresDDL) {
    table(ddl, "users");
    column(ddl, "users", "id", "integer");
    col(ddl, "users", "id").not_null = true;
    pk(ddl, "users", &["id"]);
}

fn posts(ddl: &mut PostgresDDL, references: &str) {
    table(ddl, "posts");
    column(ddl, "posts", "id", "integer");
    col(ddl, "posts", "id").not_null = true;
    column(ddl, "posts", "user_id", "integer");
    pk(ddl, "posts", &["id"]);
    fk(
        ddl,
        "posts",
        "posts_user_id_fkey",
        &["user_id"],
        references,
        &["id"],
    );
}

fn sql(prev: &PostgresDDL, cur: &PostgresDDL) -> Vec<String> {
    compute_migration(prev, cur).sql_statements
}

fn position(statements: &[String], needle: &str) -> usize {
    statements
        .iter()
        .position(|s| s.contains(needle))
        .unwrap_or_else(|| panic!("no statement containing {needle:?} in {statements:#?}"))
}

#[test]
fn dropping_fk_linked_tables_drops_the_fk_first() {
    let mut prev = PostgresDDL::new();
    users(&mut prev);
    posts(&mut prev, "users");
    let statements = sql(&prev, &PostgresDDL::new());
    assert_eq!(
        statements,
        [
            "ALTER TABLE \"posts\" DROP CONSTRAINT \"posts_user_id_fkey\";",
            "DROP TABLE \"posts\";",
            "DROP TABLE \"users\";",
        ]
    );
}

#[test]
fn table_drop_precedes_drop_of_the_unique_its_fk_references() {
    let mut prev = PostgresDDL::new();
    users(&mut prev);
    column(&mut prev, "users", "email", "text");
    unique(&mut prev, "users", "users_email_key", &["email"]);
    table(&mut prev, "invites");
    column(&mut prev, "invites", "email", "text");
    fk(
        &mut prev,
        "invites",
        "invites_email_fkey",
        &["email"],
        "users",
        &["email"],
    );
    let mut cur = PostgresDDL::new();
    users(&mut cur);

    let statements = sql(&prev, &cur);
    assert!(
        position(&statements, "DROP TABLE \"invites\"")
            < position(&statements, "DROP CONSTRAINT \"users_email_key\""),
        "{statements:#?}"
    );
}

#[test]
fn created_tables_keep_declaration_order() {
    let mut cur = PostgresDDL::new();
    let names = ["t_f", "t_a", "t_d", "t_b", "t_e", "t_c"];
    for name in names {
        table(&mut cur, name);
        column(&mut cur, name, "id", "integer");
    }
    // t_a references t_c, so t_c moves before it; the rest keep their order.
    fk(&mut cur, "t_a", "t_a_c_fkey", &["id"], "t_c", &["id"]);
    for _ in 0..5 {
        let statements = sql(&PostgresDDL::new(), &cur);
        let order: Vec<&str> = statements
            .iter()
            .filter_map(|s| s.strip_prefix("CREATE TABLE \""))
            .map(|s| &s[..3])
            .collect();
        assert_eq!(order, ["t_f", "t_d", "t_b", "t_e", "t_c", "t_a"]);
    }
}

#[test]
fn new_table_fk_to_a_unique_added_in_the_same_migration_is_deferred() {
    let mut prev = PostgresDDL::new();
    users(&mut prev);
    column(&mut prev, "users", "email", "text");
    let mut cur = prev.clone();
    unique(&mut cur, "users", "users_email_key", &["email"]);
    table(&mut cur, "invites");
    column(&mut cur, "invites", "email", "text");
    fk(
        &mut cur,
        "invites",
        "invites_email_fkey",
        &["email"],
        "users",
        &["email"],
    );

    let statements = sql(&prev, &cur);
    assert_eq!(
        statements,
        [
            "CREATE TABLE \"invites\" (\n\t\"email\" text\n);",
            "ALTER TABLE \"users\" ADD CONSTRAINT \"users_email_key\" UNIQUE (\"email\");",
            "ALTER TABLE \"invites\" ADD CONSTRAINT \"invites_email_fkey\" FOREIGN KEY (\"email\") REFERENCES \"users\"(\"email\");",
        ]
    );
}

#[test]
fn altered_view_is_dropped_before_column_changes_and_created_after() {
    let mut prev = PostgresDDL::new();
    users(&mut prev);
    column(&mut prev, "users", "legacy", "text");
    column(&mut prev, "users", "age", "integer");
    let mut view = View::new("public", "v_users");
    view.definition = Some(Cow::Borrowed("select id, legacy, age from users"));
    prev.views.push(view);

    let mut cur = PostgresDDL::new();
    users(&mut cur);
    column(&mut cur, "users", "age", "bigint");
    let mut view = View::new("public", "v_users");
    view.definition = Some(Cow::Borrowed("select id, age from users"));
    cur.views.push(view);

    let statements = sql(&prev, &cur);
    let drop_view = position(&statements, "DROP VIEW \"v_users\"");
    let create_view = position(&statements, "CREATE VIEW \"v_users\"");
    let drop_column = position(&statements, "DROP COLUMN \"legacy\"");
    let alter_type = position(&statements, "SET DATA TYPE bigint");
    assert!(
        drop_view < drop_column && drop_view < alter_type,
        "{statements:#?}"
    );
    assert!(
        create_view > drop_column && create_view > alter_type,
        "{statements:#?}"
    );
}

#[test]
fn enum_to_enum_change_recreates_the_default() {
    let mut prev = PostgresDDL::new();
    enum_type(&mut prev, "status_a", &["x", "y"]);
    enum_type(&mut prev, "status_b", &["x", "y"]);
    table(&mut prev, "items");
    enum_column(&mut prev, "items", "status", "status_a");
    col(&mut prev, "items", "status").default = Some(Cow::Borrowed("'x'"));
    let mut cur = prev.clone();
    col(&mut cur, "items", "status").sql_type = Cow::Borrowed("status_b");

    assert_eq!(
        sql(&prev, &cur),
        [
            "ALTER TABLE \"items\" ALTER COLUMN \"status\" DROP DEFAULT;",
            "ALTER TABLE \"items\" ALTER COLUMN \"status\" SET DATA TYPE status_b USING \"status\"::text::status_b;",
            "ALTER TABLE \"items\" ALTER COLUMN \"status\" SET DEFAULT 'x';",
        ]
    );
}

#[test]
fn enum_recreate_keeps_array_dimensions() {
    let mut prev = PostgresDDL::new();
    enum_type(&mut prev, "mood", &["a", "b", "c"]);
    table(&mut prev, "people");
    enum_column(&mut prev, "people", "moods", "mood");
    col(&mut prev, "people", "moods").dimensions = Some(1);
    let mut cur = prev.clone();
    cur.enums.list_mut()[0].values = Cow::Owned(vec![Cow::Borrowed("a"), Cow::Borrowed("b")]);

    assert_eq!(
        sql(&prev, &cur),
        [
            "ALTER TABLE \"people\" ALTER COLUMN \"moods\" SET DATA TYPE text[] USING \"moods\"::text[];",
            "DROP TYPE \"mood\";",
            "CREATE TYPE \"mood\" AS ENUM ('a', 'b');",
            "ALTER TABLE \"people\" ALTER COLUMN \"moods\" SET DATA TYPE \"mood\"[] USING \"moods\"::\"mood\"[];",
        ]
    );
}

#[test]
fn enum_recreate_converts_columns_that_leave_the_enum() {
    let mut prev = PostgresDDL::new();
    enum_type(&mut prev, "mood", &["a", "b", "c"]);
    table(&mut prev, "people");
    enum_column(&mut prev, "people", "m", "mood");
    let mut cur = prev.clone();
    cur.enums.list_mut()[0].values = Cow::Owned(vec![Cow::Borrowed("a"), Cow::Borrowed("b")]);
    let m = col(&mut cur, "people", "m");
    m.sql_type = Cow::Borrowed("text");
    m.type_schema = None;

    let statements = sql(&prev, &cur);
    assert_eq!(
        statements[..3],
        [
            "ALTER TABLE \"people\" ALTER COLUMN \"m\" SET DATA TYPE text USING \"m\"::text;",
            "DROP TYPE \"mood\";",
            "CREATE TYPE \"mood\" AS ENUM ('a', 'b');",
        ]
    );
    assert!(
        statements
            .iter()
            .all(|s| !s.contains("USING \"m\"::\"mood\"")),
        "{statements:#?}"
    );
}

#[test]
fn serial_transitions_manage_the_sequence() {
    let serial_case = |from: &str, to: &str| {
        let mut prev = PostgresDDL::new();
        table(&mut prev, "s");
        column(&mut prev, "s", "n", from);
        col(&mut prev, "s", "n").not_null = true;
        let mut cur = prev.clone();
        col(&mut cur, "s", "n").sql_type = Cow::Owned(to.to_string());
        sql(&prev, &cur)
    };

    assert_eq!(
        serial_case("integer", "serial"),
        [
            "CREATE SEQUENCE \"s_n_seq\" AS integer;",
            "ALTER TABLE \"s\" ALTER COLUMN \"n\" SET DEFAULT nextval('\"s_n_seq\"');",
            "ALTER SEQUENCE \"s_n_seq\" OWNED BY \"s\".\"n\";",
        ]
    );
    assert_eq!(
        serial_case("serial", "integer"),
        [
            "ALTER TABLE \"s\" ALTER COLUMN \"n\" DROP DEFAULT;",
            "DROP SEQUENCE \"s_n_seq\";",
        ]
    );
    assert_eq!(
        serial_case("serial", "bigserial"),
        [
            "ALTER TABLE \"s\" ALTER COLUMN \"n\" SET DATA TYPE bigint USING \"n\"::bigint;",
            "ALTER SEQUENCE \"s_n_seq\" AS bigint;",
        ]
    );
    assert!(
        serial_case("SERIAL", "serial").is_empty(),
        "spelling-only change"
    );
}

#[test]
fn generated_expression_change_recreates_the_column() {
    let mut prev = PostgresDDL::new();
    table(&mut prev, "g");
    column(&mut prev, "g", "a", "integer");
    column(&mut prev, "g", "b", "integer");
    col(&mut prev, "g", "b").generated = Some(Generated {
        expression: Cow::Borrowed("a + 1"),
        gen_type: GeneratedType::Stored,
    });
    let mut cur = prev.clone();
    col(&mut cur, "g", "b").generated = Some(Generated {
        expression: Cow::Borrowed("a + 2"),
        gen_type: GeneratedType::Stored,
    });

    assert_eq!(
        sql(&prev, &cur),
        [
            "ALTER TABLE \"g\" DROP COLUMN \"b\";",
            "ALTER TABLE \"g\" ADD COLUMN \"b\" integer GENERATED ALWAYS AS (a + 2) STORED;",
        ]
    );

    // Formatting-only differences are not a change.
    let mut same = prev.clone();
    col(&mut same, "g", "b").generated = Some(Generated {
        expression: Cow::Borrowed("(a  +  1)"),
        gen_type: GeneratedType::Stored,
    });
    assert!(sql(&prev, &same).is_empty());
}

#[test]
fn generated_column_type_change_has_no_using_clause() {
    let mut prev = PostgresDDL::new();
    table(&mut prev, "g");
    column(&mut prev, "g", "a", "integer");
    column(&mut prev, "g", "b", "integer");
    col(&mut prev, "g", "b").generated = Some(Generated {
        expression: Cow::Borrowed("a + 1"),
        gen_type: GeneratedType::Stored,
    });
    let mut cur = prev.clone();
    col(&mut cur, "g", "b").sql_type = Cow::Borrowed("bigint");

    assert_eq!(
        sql(&prev, &cur),
        ["ALTER TABLE \"g\" ALTER COLUMN \"b\" SET DATA TYPE bigint;"]
    );
}

#[test]
fn making_a_column_generated_recreates_its_indexes_and_constraints() {
    let mut prev = PostgresDDL::new();
    table(&mut prev, "g");
    column(&mut prev, "g", "a", "integer");
    column(&mut prev, "g", "b", "integer");
    prev.indexes.push(Index::new(
        "public",
        "g",
        "g_b_idx",
        vec![IndexColumn::new("b")],
    ));
    unique(&mut prev, "g", "g_b_key", &["b"]);
    table(&mut prev, "refs");
    column(&mut prev, "refs", "b", "integer");
    fk(&mut prev, "refs", "refs_b_fkey", &["b"], "g", &["b"]);
    let mut cur = prev.clone();
    col(&mut cur, "g", "b").generated = Some(Generated {
        expression: Cow::Borrowed("a + 1"),
        gen_type: GeneratedType::Stored,
    });

    assert_eq!(
        sql(&prev, &cur),
        [
            "ALTER TABLE \"refs\" DROP CONSTRAINT \"refs_b_fkey\";",
            "ALTER TABLE \"g\" DROP COLUMN \"b\";",
            "ALTER TABLE \"g\" ADD COLUMN \"b\" integer GENERATED ALWAYS AS (a + 1) STORED;",
            "ALTER TABLE \"g\" ADD CONSTRAINT \"g_b_key\" UNIQUE (\"b\");",
            "CREATE INDEX \"g_b_idx\" ON \"g\"(\"b\");",
            "ALTER TABLE \"refs\" ADD CONSTRAINT \"refs_b_fkey\" FOREIGN KEY (\"b\") REFERENCES \"g\"(\"b\");",
        ]
    );
}

#[test]
fn row_level_security_follows_the_table_and_its_policies() {
    let mut prev = PostgresDDL::new();
    users(&mut prev);

    // Explicit toggle on an existing table.
    let mut cur = prev.clone();
    cur.tables.list_mut()[0].is_rls_enabled = Some(true);
    assert_eq!(
        sql(&prev, &cur),
        ["ALTER TABLE \"users\" ENABLE ROW LEVEL SECURITY;"]
    );
    assert_eq!(
        sql(&cur, &prev),
        ["ALTER TABLE \"users\" DISABLE ROW LEVEL SECURITY;"]
    );

    // The first policy turns it on, removing the last one turns it off.
    let mut with_policy = prev.clone();
    let mut policy = Policy::new("public", "users", "p1");
    policy.using = Some(Cow::Borrowed("true"));
    with_policy.policies.push(policy);
    assert_eq!(
        sql(&prev, &with_policy),
        [
            "ALTER TABLE \"users\" ENABLE ROW LEVEL SECURITY;",
            "CREATE POLICY \"p1\" ON \"users\" AS PERMISSIVE USING (true);",
        ]
    );
    assert_eq!(
        sql(&with_policy, &prev),
        [
            "DROP POLICY \"p1\" ON \"users\";",
            "ALTER TABLE \"users\" DISABLE ROW LEVEL SECURITY;",
        ]
    );

    // A new table with a policy gets RLS on creation.
    let statements = sql(&PostgresDDL::new(), &with_policy);
    assert!(
        statements.contains(&"ALTER TABLE \"users\" ENABLE ROW LEVEL SECURITY;".to_string()),
        "{statements:#?}"
    );
}

#[test]
fn type_modifier_and_enum_name_changes_are_detected() {
    let mut prev = PostgresDDL::new();
    enum_type(&mut prev, "time_unit", &["s"]);
    enum_type(&mut prev, "time_zone", &["utc"]);
    table(&mut prev, "ev");
    column(&mut prev, "ev", "at", "timestamp(3)");
    column(&mut prev, "ev", "at_tz", "timestamptz(3)");
    enum_column(&mut prev, "ev", "unit", "time_unit");
    let mut cur = prev.clone();
    col(&mut cur, "ev", "at").sql_type = Cow::Borrowed("timestamp(6)");
    col(&mut cur, "ev", "at_tz").sql_type = Cow::Borrowed("timestamp(0) with time zone");
    col(&mut cur, "ev", "unit").sql_type = Cow::Borrowed("time_zone");

    assert_eq!(
        sql(&prev, &cur),
        [
            "ALTER TABLE \"ev\" ALTER COLUMN \"at\" SET DATA TYPE timestamp(6) USING \"at\"::timestamp(6);",
            "ALTER TABLE \"ev\" ALTER COLUMN \"at_tz\" SET DATA TYPE timestamp(0) with time zone USING \"at_tz\"::timestamp(0) with time zone;",
            "ALTER TABLE \"ev\" ALTER COLUMN \"unit\" SET DATA TYPE time_zone USING \"unit\"::text::time_zone;",
        ]
    );

    // Same type, different spelling: no change.
    let mut alias = prev.clone();
    col(&mut alias, "ev", "at_tz").sql_type = Cow::Borrowed("timestamp(3) with time zone");
    assert!(sql(&prev, &alias).is_empty());
}

#[test]
fn collation_change_alters_the_column_type() {
    let mut prev = PostgresDDL::new();
    users(&mut prev);
    column(&mut prev, "users", "name", "text");
    let mut cur = prev.clone();
    col(&mut cur, "users", "name").collate = Some(Cow::Borrowed("C"));
    assert_eq!(
        sql(&prev, &cur),
        ["ALTER TABLE \"users\" ALTER COLUMN \"name\" SET DATA TYPE text COLLATE \"C\";"]
    );
    assert_eq!(
        sql(&cur, &prev),
        ["ALTER TABLE \"users\" ALTER COLUMN \"name\" SET DATA TYPE text COLLATE \"default\";"]
    );
}

#[test]
fn comment_containing_semicolon_newline_stays_one_statement() {
    let mut prev = PostgresDDL::new();
    users(&mut prev);
    let mut cur = prev.clone();
    col(&mut cur, "users", "id").comment = Some(Cow::Borrowed("first;\nsecond"));
    assert_eq!(
        sql(&prev, &cur),
        ["COMMENT ON COLUMN \"users\".\"id\" IS 'first;\nsecond';"]
    );

    let mut added = prev.clone();
    column(&mut added, "users", "note", "text");
    col(&mut added, "users", "note").comment = Some(Cow::Borrowed("a';\nb"));
    assert_eq!(
        sql(&prev, &added),
        [
            "ALTER TABLE \"users\" ADD COLUMN \"note\" text;",
            "COMMENT ON COLUMN \"users\".\"note\" IS 'a'';\nb';",
        ]
    );
}

#[test]
fn concurrent_index_recreate_drops_without_concurrently() {
    let mut prev = PostgresDDL::new();
    users(&mut prev);
    let mut index = Index::new(
        "public",
        "users",
        "users_id_idx",
        vec![IndexColumn::new("id")],
    );
    index.concurrently = true;
    prev.indexes.push(index);
    let mut cur = prev.clone();
    cur.indexes.list_mut()[0].where_clause = Some(Cow::Borrowed("id > 0"));
    assert_eq!(
        sql(&prev, &cur),
        [
            "DROP INDEX \"users_id_idx\";",
            "CREATE INDEX CONCURRENTLY \"users_id_idx\" ON \"users\"(\"id\") WHERE id > 0;",
        ]
    );
}

#[test]
fn equivalent_spellings_do_not_produce_alters() {
    let mut prev = PostgresDDL::new();
    users(&mut prev);
    column(&mut prev, "users", "balance", "int4");
    col(&mut prev, "users", "balance").default = Some(Cow::Borrowed("'-1'::integer"));
    column(&mut prev, "users", "active", "bool");
    col(&mut prev, "users", "active").default = Some(Cow::Borrowed("true"));
    column(&mut prev, "users", "score", "float8");
    column(&mut prev, "users", "ratio", "float4");
    column(&mut prev, "users", "price", "numeric(10,0)");
    column(&mut prev, "users", "status", "status");
    col(&mut prev, "users", "status").type_schema = Some(Cow::Borrowed("app"));
    col(&mut prev, "users", "status").default = Some(Cow::Borrowed("'on'::app.status"));
    let mut index = Index::new(
        "public",
        "users",
        "users_active_idx",
        vec![IndexColumn::new("active")],
    );
    index.where_clause = Some(Cow::Borrowed("(active IS NOT NULL)"));
    prev.indexes.push(index);

    let mut cur = PostgresDDL::new();
    users(&mut cur);
    column(&mut cur, "users", "balance", "INTEGER");
    col(&mut cur, "users", "balance").default = Some(Cow::Borrowed("-1"));
    column(&mut cur, "users", "active", "BOOLEAN");
    col(&mut cur, "users", "active").default = Some(Cow::Borrowed("TRUE"));
    column(&mut cur, "users", "score", "DOUBLE PRECISION");
    column(&mut cur, "users", "ratio", "REAL");
    column(&mut cur, "users", "price", "numeric(10)");
    column(&mut cur, "users", "status", "status");
    col(&mut cur, "users", "status").type_schema = Some(Cow::Borrowed("app"));
    col(&mut cur, "users", "status").default = Some(Cow::Borrowed("'on'"));
    let mut index = Index::new(
        "public",
        "users",
        "users_active_idx",
        vec![IndexColumn::new("active")],
    );
    index.where_clause = Some(Cow::Borrowed("active IS NOT NULL"));
    index.concurrently = true;
    index.name_explicit = false;
    cur.indexes.push(index);

    let statements = sql(&prev, &cur);
    assert!(statements.is_empty(), "{statements:#?}");
}

#[test]
fn index_nulls_order_is_rendered_and_compared() {
    let mut prev = PostgresDDL::new();
    users(&mut prev);
    column(&mut prev, "users", "score", "integer");
    let mut cur = prev.clone();
    cur.indexes.push(Index::new(
        "public",
        "users",
        "users_score_idx",
        vec![IndexColumn::new("score").desc().nulls_last()],
    ));
    assert_eq!(
        sql(&prev, &cur),
        ["CREATE INDEX \"users_score_idx\" ON \"users\"(\"score\" DESC NULLS LAST);"]
    );

    let mut nulls_first = cur.clone();
    nulls_first.indexes.list_mut()[0].columns[0] = IndexColumn::new("score").desc();
    assert_eq!(
        sql(&cur, &nulls_first),
        [
            "DROP INDEX \"users_score_idx\";",
            "CREATE INDEX \"users_score_idx\" ON \"users\"(\"score\" DESC);",
        ]
    );
}
