use drizzle_migrations::postgres::{
    PostgresDDL, diff::compute_migration as compute_postgres_migration,
};
use drizzle_migrations::sqlite::{SQLiteDDL, diff::compute_migration as compute_sqlite_migration};
use drizzle_types::{postgres::ddl as pg, sqlite::ddl as sqlite};
use std::borrow::Cow;

#[test]
fn sqlite_table_conversion_for_options_collate_and_named_unique() {
    let table = sqlite::TableDef::new("sqlite_parity")
        .strict()
        .without_rowid()
        .into_table();
    let columns = [
        sqlite::ColumnDef::new("sqlite_parity", "key", "TEXT")
            .primary_key()
            .into_column(),
        sqlite::ColumnDef::new("sqlite_parity", "email", "TEXT")
            .collate("NOCASE")
            .into_column(),
    ];
    const PK_COLS: &[Cow<'static, str>] = &[Cow::Borrowed("key")];
    const UNIQUE_COLS: &[Cow<'static, str>] = &[Cow::Borrowed("email")];
    let pk = sqlite::PrimaryKeyDef::new("sqlite_parity", "sqlite_parity_pk")
        .columns(PK_COLS)
        .into_primary_key();
    let unique = sqlite::UniqueConstraintDef::new("sqlite_parity", "sqlite_parity_email_unique")
        .columns(UNIQUE_COLS)
        .explicit_name()
        .into_unique_constraint();
    let mut cur = SQLiteDDL::default();
    cur.tables.push(table);
    cur.columns.list_mut().extend(columns);
    cur.pks.push(pk);
    cur.uniques.push(unique);
    let migration_sql =
        compute_sqlite_migration(&SQLiteDDL::default(), &cur).sql_statements[0].clone();

    assert_eq!(
        migration_sql,
        "CREATE TABLE `sqlite_parity` (\n\t`key` TEXT PRIMARY KEY NOT NULL,\n\t`email` TEXT COLLATE NOCASE,\n\tCONSTRAINT `sqlite_parity_email_unique` UNIQUE(`email`)\n) WITHOUT ROWID, STRICT;"
    );
}

/// Parity with the compile-time emitter (`procmacros/src/sqlite/table/ddl.rs`):
/// for the same logical schema, the runtime renderer must produce byte-for-byte
/// the SQL that the `#[SQLiteTable]` macro stores in its `SQLSchema::SQL`
/// const. The macro emits, for a table with an INTEGER PK AUTOINCREMENT
/// column, a generated column, and a single-column FK:
///
/// - backtick-quoted identifiers
/// - inline `PRIMARY KEY AUTOINCREMENT` on the PK column, no `NOT NULL`
///   (INTEGER PK aliases rowid)
/// - `GENERATED ALWAYS AS (expr) VIRTUAL|STORED` with exactly one paren layer
/// - `CONSTRAINT `fk_{table}_{col}_{ref_table}_{ref_col}_fk` FOREIGN KEY ...`
///   with `NO ACTION` omitted
#[test]
fn sqlite_runtime_renderer_matches_macro_const_sql_format() {
    let mut cur = SQLiteDDL::default();
    cur.tables.push(sqlite::Table::new("users"));
    // Columns shaped like the macro's runtime to_snapshot: no PK flags on the
    // column, one PrimaryKey entity, generated expression pre-parenthesized.
    cur.columns.push(
        sqlite::Column::new("users", "id", "integer")
            .not_null()
            .autoincrement(),
    );
    cur.columns
        .push(sqlite::Column::new("users", "name", "text").not_null());
    let mut name_len = sqlite::Column::new("users", "name_len", "integer");
    name_len.generated = Some(sqlite::Generated {
        expression: Cow::Borrowed("(length(name))"),
        gen_type: sqlite::GeneratedType::Virtual,
    });
    cur.columns.push(name_len);
    cur.columns
        .push(sqlite::Column::new("users", "group_id", "integer").not_null());
    cur.pks.push(sqlite::PrimaryKey::from_strings(
        "users".to_string(),
        "users_pk".to_string(),
        vec!["id".to_string()],
    ));
    cur.fks.push(
        sqlite::ForeignKey::from_strings(
            "users".to_string(),
            "fk_users_group_id_groups_id_fk".to_string(),
            vec!["group_id".to_string()],
            "groups".to_string(),
            vec!["id".to_string()],
        )
        .on_delete("CASCADE")
        .on_update("NO ACTION"),
    );

    let migration_sql =
        compute_sqlite_migration(&SQLiteDDL::default(), &cur).sql_statements[0].clone();

    assert_eq!(
        migration_sql,
        "CREATE TABLE `users` (\n\
         \t`id` INTEGER PRIMARY KEY AUTOINCREMENT,\n\
         \t`name` TEXT NOT NULL,\n\
         \t`name_len` INTEGER GENERATED ALWAYS AS (length(name)) VIRTUAL,\n\
         \t`group_id` INTEGER NOT NULL,\n\
         \tCONSTRAINT `fk_users_group_id_groups_id_fk` FOREIGN KEY (`group_id`) REFERENCES `groups`(`id`) ON DELETE CASCADE\n\
         );"
    );
}

/// Same parity contract for column-flag-only DDL (`ColumnDef::primary_key()
/// .autoincrement()` without a PrimaryKey entity): the flag must render an
/// inline PRIMARY KEY exactly like the macro emitter does.
#[test]
fn sqlite_column_flag_pk_matches_macro_const_sql_format() {
    let mut cur = SQLiteDDL::default();
    cur.tables
        .push(sqlite::TableDef::new("flagged").into_table());
    cur.columns.push(
        sqlite::ColumnDef::new("flagged", "id", "INTEGER")
            .primary_key()
            .autoincrement()
            .into_column(),
    );
    cur.columns.push(
        sqlite::ColumnDef::new("flagged", "name", "TEXT")
            .not_null()
            .into_column(),
    );

    let migration_sql =
        compute_sqlite_migration(&SQLiteDDL::default(), &cur).sql_statements[0].clone();

    assert_eq!(
        migration_sql,
        "CREATE TABLE `flagged` (\n\t`id` INTEGER PRIMARY KEY AUTOINCREMENT,\n\t`name` TEXT NOT NULL\n);"
    );
}

/// Generated STORED columns must be parenthesized identically whether the
/// expression arrives bare (introspection) or pre-parenthesized (macro).
#[test]
fn sqlite_generated_column_paren_parity() {
    let render = |expression: &str| {
        let mut cur = SQLiteDDL::default();
        cur.tables.push(sqlite::Table::new("g"));
        let mut col = sqlite::Column::new("g", "value", "integer").not_null();
        col.generated = Some(sqlite::Generated {
            expression: expression.to_string().into(),
            gen_type: sqlite::GeneratedType::Stored,
        });
        cur.columns.push(col);
        compute_sqlite_migration(&SQLiteDDL::default(), &cur).sql_statements[0].clone()
    };

    let expected =
        "CREATE TABLE `g` (\n\t`value` INTEGER GENERATED ALWAYS AS (length(x)) STORED NOT NULL\n);";
    assert_eq!(render("length(x)"), expected);
    assert_eq!(render("(length(x))"), expected);
}

#[test]
fn postgres_index_conversion_for_concurrently_where_and_opclass() {
    let table = pg::Table::new("public", "users");
    let column = pg::Column::new("public", "users", "email", "TEXT");
    let mut index = pg::Index::new(
        "public",
        "users",
        "users_email_idx",
        vec![pg::IndexColumn::new("email").with_opclass(pg::Opclass::new("text_pattern_ops"))],
    )
    .unique();
    index.concurrently = true;
    index.method = Some(Cow::Borrowed("btree"));
    index.where_clause = Some(Cow::Borrowed("email IS NOT NULL"));
    let mut prev = PostgresDDL::new();
    prev.tables.push(table.clone());
    prev.columns.push(column.clone());
    let mut cur = prev.clone();
    cur.indexes.push(index);
    let migration_sql = compute_postgres_migration(&prev, &cur).sql_statements[0].clone();

    assert_eq!(
        migration_sql,
        "CREATE UNIQUE INDEX CONCURRENTLY \"users_email_idx\" ON \"users\" USING btree(\"email\" text_pattern_ops) WHERE email IS NOT NULL;"
    );
}

#[test]
fn postgres_policy_conversion_for_public_role_and_as_clause() {
    let table = pg::Table::new("public", "users");
    let column = pg::Column::new("public", "users", "id", "INTEGER");
    let mut policy = pg::Policy::new("public", "users", "users_policy");
    policy.as_clause = Some(Cow::Borrowed("permissive"));
    policy.for_clause = Some(Cow::Borrowed("select"));
    policy.to = Some(vec![Cow::Borrowed("public")]);
    policy.using = Some(Cow::Borrowed("id > 0"));
    let mut prev = PostgresDDL::new();
    prev.tables.push(table.clone());
    prev.columns.push(column);
    let mut cur = prev.clone();
    cur.policies.push(policy);
    let migration_sql = compute_postgres_migration(&prev, &cur).sql_statements[0].clone();

    assert_eq!(
        migration_sql,
        "CREATE POLICY \"users_policy\" ON \"users\" AS PERMISSIVE FOR SELECT TO PUBLIC USING (id > 0);"
    );
}

#[test]
fn postgres_column_conversion_for_identity_generated_and_collate() {
    let table = pg::Table::new("public", "columns_parity");
    let mut identity = pg::Identity::by_default("columns_parity_id_seq");
    identity.increment = Some(Cow::Borrowed("5"));
    identity.start_with = Some(Cow::Borrowed("10"));

    let columns = [
        pg::Column::new("public", "columns_parity", "id", "INTEGER").identity(identity),
        {
            let mut col = pg::Column::new("public", "columns_parity", "name", "TEXT");
            col.collate = Some(Cow::Borrowed("C"));
            col
        },
        {
            let mut col = pg::Column::new("public", "columns_parity", "name_len", "INTEGER");
            col.default = Some(Cow::Borrowed("0"));
            col.generated = Some(pg::Generated {
                expression: Cow::Borrowed("length(name)"),
                gen_type: pg::GeneratedType::Stored,
            });
            col
        },
    ];

    let mut cur = PostgresDDL::new();
    cur.tables.push(table);
    cur.columns.list_mut().extend(columns);
    let migration_sql =
        compute_postgres_migration(&PostgresDDL::new(), &cur).sql_statements[0].clone();

    assert_eq!(
        migration_sql,
        "CREATE TABLE \"columns_parity\" (\n\t\"id\" INTEGER GENERATED BY DEFAULT AS IDENTITY (INCREMENT BY 5 START WITH 10),\n\t\"name\" TEXT COLLATE \"C\",\n\t\"name_len\" INTEGER GENERATED ALWAYS AS (length(name)) STORED\n);"
    );
}

#[test]
fn postgres_column_conversion_for_arrays_and_comments() {
    let table = pg::Table::new("public", "array_comment_parity").comment("It's documented");
    let columns = [
        {
            let mut col = pg::Column::new("public", "array_comment_parity", "numbers", "INTEGER")
                .not_null()
                .comment("Integer values");
            col.dimensions = Some(1);
            col
        },
        {
            let mut col = pg::Column::new("public", "array_comment_parity", "tags", "TEXT")
                .comment("Text values");
            col.dimensions = Some(1);
            col
        },
    ];
    let mut cur = PostgresDDL::new();
    cur.tables.push(table);
    cur.columns.list_mut().extend(columns);
    let migration_sql = compute_postgres_migration(&PostgresDDL::new(), &cur).sql_statements;

    assert_eq!(
        migration_sql,
        vec![
            "CREATE TABLE \"array_comment_parity\" (\n\t\"numbers\" INTEGER[] NOT NULL,\n\t\"tags\" TEXT[]\n);",
            "COMMENT ON TABLE \"array_comment_parity\" IS 'It''s documented';",
            "COMMENT ON COLUMN \"array_comment_parity\".\"numbers\" IS 'Integer values';",
            "COMMENT ON COLUMN \"array_comment_parity\".\"tags\" IS 'Text values';",
        ]
    );
}

#[test]
fn postgres_table_conversion_for_storage_attrs() {
    let table = pg::Table::new("public", "storage_parity")
        .unlogged()
        .inherits("parent_table")
        .tablespace("fast_storage");
    let columns = [pg::Column::new("public", "storage_parity", "id", "INTEGER").not_null()];
    let mut cur = PostgresDDL::new();
    cur.tables.push(table);
    cur.columns.list_mut().extend(columns);
    let migration_sql =
        compute_postgres_migration(&PostgresDDL::new(), &cur).sql_statements[0].clone();

    assert_eq!(
        migration_sql,
        "CREATE UNLOGGED TABLE \"storage_parity\" (\n\t\"id\" INTEGER NOT NULL\n) INHERITS (\"parent_table\") TABLESPACE \"fast_storage\";"
    );
}

#[test]
fn postgres_table_conversion_for_deferrable_fk_and_composite_unique() {
    let parent = pg::Table::new("public", "constraint_parent");
    let child = pg::Table::new("public", "constraint_child");
    let columns = [
        pg::Column::new("public", "constraint_child", "tenant_id", "INTEGER").not_null(),
        pg::Column::new("public", "constraint_child", "parent_id", "INTEGER").not_null(),
        pg::Column::new("public", "constraint_child", "slug", "TEXT").not_null(),
    ];
    let fk = pg::ForeignKey::from_strings(
        "public".to_string(),
        "constraint_child".to_string(),
        "constraint_child_tenant_parent_fkey".to_string(),
        vec!["tenant_id".to_string(), "parent_id".to_string()],
        "public".to_string(),
        "constraint_parent".to_string(),
        vec!["tenant_id".to_string(), "id".to_string()],
    )
    .deferrable()
    .initially_deferred();
    let unique = pg::UniqueConstraint::from_strings(
        "public".to_string(),
        "constraint_child".to_string(),
        "constraint_child_tenant_slug_key".to_string(),
        vec!["tenant_id".to_string(), "slug".to_string()],
    )
    .deferrable()
    .initially_deferred();
    let mut cur = PostgresDDL::new();
    cur.tables.push(parent);
    cur.tables.push(child);
    cur.columns.push(pg::Column::new(
        "public",
        "constraint_parent",
        "tenant_id",
        "INTEGER",
    ));
    cur.columns.push(pg::Column::new(
        "public",
        "constraint_parent",
        "id",
        "INTEGER",
    ));
    cur.columns.list_mut().extend(columns);
    cur.fks.push(fk);
    cur.uniques.push(unique);

    let migration_sql = compute_postgres_migration(&PostgresDDL::new(), &cur)
        .sql_statements
        .into_iter()
        .find(|sql| sql.contains("CREATE TABLE \"constraint_child\""))
        .expect("child create table sql");

    assert_eq!(
        migration_sql,
        "CREATE TABLE \"constraint_child\" (\n\t\"tenant_id\" INTEGER NOT NULL,\n\t\"parent_id\" INTEGER NOT NULL,\n\t\"slug\" TEXT NOT NULL,\n\tCONSTRAINT \"constraint_child_tenant_parent_fkey\" FOREIGN KEY (\"tenant_id\", \"parent_id\") REFERENCES \"constraint_parent\"(\"tenant_id\", \"id\") DEFERRABLE INITIALLY DEFERRED,\n\tCONSTRAINT \"constraint_child_tenant_slug_key\" UNIQUE(\"tenant_id\", \"slug\") DEFERRABLE INITIALLY DEFERRED\n);"
    );
}

#[test]
fn postgres_view_conversion_for_materialized_options() {
    let with = pg::ViewWithOption {
        security_barrier: Some(true),
        check_option: Some(Cow::Borrowed("local")),
        ..Default::default()
    };

    let mut view = pg::View::new("public", "active_users");
    view.materialized = true;
    view.definition = Some(Cow::Borrowed("SELECT id FROM users WHERE active"));
    view.with = Some(with);
    view.using = Some(Cow::Borrowed("heap"));
    view.tablespace = Some(Cow::Borrowed("fast_storage"));
    view.with_no_data = Some(true);
    let mut cur = PostgresDDL::new();
    cur.views.push(view);
    let migration_sql =
        compute_postgres_migration(&PostgresDDL::new(), &cur).sql_statements[0].clone();

    assert_eq!(
        migration_sql,
        "CREATE MATERIALIZED VIEW \"active_users\" USING heap WITH (security_barrier = true) TABLESPACE \"fast_storage\" AS SELECT id FROM users WHERE active WITH LOCAL CHECK OPTION WITH NO DATA;"
    );
}
