//! MySQL introspection-codegen round-trip coverage.

use drizzle_migrations::{
    Snapshot,
    mysql::{
        CheckConstraint, Column, ForeignKey, Generated, Index, IndexAlgorithm, IndexColumn,
        IndexLock, IndexMethod, InlineEnum, InlineType, MySQLDDL, PrimaryKey, ReferentialAction,
        Table, TableOption, UniqueConstraint, View, ViewAlgorithm, ViewCheckOption,
        ViewSqlSecurity,
        codegen::{CodegenError, CodegenOptions, FieldCasing, generate_rust_schema},
        compute_migration,
    },
    parser::SchemaParser,
};
use drizzle_types::Dialect;

const DATABASE: &str = "app";

fn app_table(name: &str) -> Table {
    let mut table = Table::new(name.to_owned());
    table.database = Some(DATABASE.to_owned().into());
    table
}

fn app_column(table: &str, name: &str, sql_type: &str) -> Column {
    let mut column = Column::new(table.to_owned(), name.to_owned(), sql_type.to_owned());
    column.database = Some(DATABASE.to_owned().into());
    column
}

fn app_primary_key(table: &str, columns: &[&str]) -> PrimaryKey {
    let mut key = PrimaryKey::new(
        table.to_owned(),
        columns.iter().map(|column| (*column).to_owned()),
    );
    key.database = Some(DATABASE.to_owned().into());
    // `PRIMARY` is the normal MySQL name, but the macro-derived snapshot does
    // not carry it as an explicit name.
    key.name = None;
    key
}

fn app_unique(table: &str, name: &str, columns: &[&str]) -> UniqueConstraint {
    let mut unique = UniqueConstraint::new(
        table.to_owned(),
        name.to_owned(),
        columns.iter().map(|column| (*column).to_owned()),
    );
    unique.database = Some(DATABASE.to_owned().into());
    unique
}

fn app_check(table: &str, name: &str, expression: &str) -> CheckConstraint {
    let mut check = CheckConstraint::new(table.to_owned(), name.to_owned(), expression.to_owned());
    check.database = Some(DATABASE.to_owned().into());
    check
}

fn app_foreign_key(
    table: &str,
    name: &str,
    columns: &[&str],
    foreign_table: &str,
    foreign_columns: &[&str],
) -> ForeignKey {
    let mut foreign_key = ForeignKey::new(
        table.to_owned(),
        name.to_owned(),
        columns.iter().map(|column| (*column).to_owned()),
        foreign_table.to_owned(),
        foreign_columns.iter().map(|column| (*column).to_owned()),
    );
    foreign_key.database = Some(DATABASE.to_owned().into());
    foreign_key.foreign_database = Some(DATABASE.to_owned().into());
    foreign_key
}

fn rich_ddl() -> MySQLDDL {
    let mut ddl = MySQLDDL::new();

    let mut accounts = app_table("accounts");
    accounts.engine = Some("InnoDB".into());
    accounts.charset = Some("utf8mb4".into());
    accounts.collation = Some("utf8mb4_0900_ai_ci".into());
    accounts.comment = Some("account records".into());
    ddl.tables.push(accounts);

    let mut account_id = app_column("accounts", "id", "BIGINT UNSIGNED");
    account_id.not_null = true;
    account_id.autoincrement = true;
    account_id.primary_key = true;
    ddl.columns.push(account_id);

    let mut email = app_column("accounts", "email", "VARCHAR(255)");
    email.not_null = true;
    email.default = Some("'guest@example.test'".into());
    ddl.columns.push(email);

    let mut handle = app_column("accounts", "handle", "VARCHAR(64)");
    handle.not_null = true;
    handle.unique = true;
    ddl.columns.push(handle);

    let mut status = app_column("accounts", "status", "ENUM('Draft', 'Published')");
    status.not_null = true;
    status.default = Some("'Draft'".into());
    status.inline_type = Some(InlineType::Enum(InlineEnum::new(["Draft", "Published"])));
    status.charset = Some("utf8mb4".into());
    status.collation = Some("utf8mb4_bin".into());
    status.comment = Some("account status".into());
    ddl.columns.push(status);

    let mut roles = app_column("accounts", "roles", "SET('reader', 'writer')");
    roles.inline_type = Some(InlineType::Set(InlineEnum::new(["reader", "writer"])));
    ddl.columns.push(roles);

    let mut login_count = app_column("accounts", "login_count", "INT");
    login_count.not_null = true;
    login_count.default = Some("0".into());
    ddl.columns.push(login_count);

    let mut updated_at = app_column("accounts", "updated_at", "TIMESTAMP");
    updated_at.not_null = true;
    updated_at.default = Some("CURRENT_TIMESTAMP".into());
    updated_at.on_update = Some("CURRENT_TIMESTAMP".into());
    ddl.columns.push(updated_at);

    let mut roles_length = app_column("accounts", "roles_length", "INT");
    roles_length.not_null = true;
    roles_length.generated = Some(Generated::virtual_column("CHAR_LENGTH(roles)"));
    ddl.columns.push(roles_length);

    ddl.pks.push(app_primary_key("accounts", &["id"]));
    ddl.uniques
        .push(app_unique("accounts", "accounts_email_key", &["email"]));
    ddl.checks.push(app_check(
        "accounts",
        "accounts_login_count_check",
        "login_count >= 0",
    ));

    let mut status_index = Index::new(
        "accounts",
        "accounts_status_idx",
        vec![IndexColumn::column("status")],
    );
    status_index.database = Some(DATABASE.into());
    status_index.using = Some(IndexMethod::Hash);
    status_index.algorithm = Some(IndexAlgorithm::Inplace);
    status_index.lock = Some(IndexLock::None);
    ddl.indexes.push(status_index);

    ddl.tables.push(app_table("orders"));
    let mut order_id = app_column("orders", "id", "INT");
    order_id.not_null = true;
    order_id.primary_key = true;
    ddl.columns.push(order_id);
    let mut account_id = app_column("orders", "account_id", "BIGINT UNSIGNED");
    account_id.not_null = true;
    ddl.columns.push(account_id);
    ddl.pks.push(app_primary_key("orders", &["id"]));

    let mut account_foreign_key = app_foreign_key(
        "orders",
        "orders_account_id_fkey",
        &["account_id"],
        "accounts",
        &["id"],
    );
    account_foreign_key.on_delete = Some(ReferentialAction::Cascade);
    account_foreign_key.on_update = Some(ReferentialAction::Restrict);
    ddl.fks.push(account_foreign_key);

    let mut view = View::new(
        "active_accounts",
        "SELECT id FROM accounts WHERE login_count > 0",
    );
    view.database = Some(DATABASE.into());
    view.algorithm = Some(ViewAlgorithm::Merge);
    view.sql_security = Some(ViewSqlSecurity::Invoker);
    view.check_option = Some(ViewCheckOption::Cascaded);
    ddl.views.push(view);

    MySQLDDL::try_from_entities(ddl.to_entities()).expect("rich test DDL is valid")
}

fn parse_generated_ddl(source: &str) -> MySQLDDL {
    let parsed = SchemaParser::parse(source);
    assert!(
        parsed.errors.is_empty(),
        "generated source:\n{source}\nerrors: {:#?}",
        parsed.errors
    );
    assert_eq!(parsed.dialect, Dialect::MySQL);

    let Snapshot::MySQL(snapshot) = Snapshot::from_parse_result(&parsed, Dialect::MySQL, None)
    else {
        panic!("MySQL parser output must build a MySQL snapshot");
    };
    MySQLDDL::try_from_entities(snapshot.ddl).expect("parser-produced MySQL DDL is valid")
}

#[test]
fn mysql_codegen_round_trips_macro_representable_ddl() {
    let original = rich_ddl();
    let generated = generate_rust_schema(
        &original,
        &CodegenOptions {
            include_schema: true,
            schema_name: "ApplicationSchema".into(),
            use_pub: true,
            ..CodegenOptions::default()
        },
    )
    .expect("rich DDL is representable");

    assert!(
        generated.warnings.is_empty(),
        "warnings: {:#?}",
        generated.warnings
    );
    assert!(generated.code.contains("MySQLEnum"));
    assert!(generated.code.contains("SET(\"reader\", \"writer\")"));
    assert!(
        generated
            .code
            .contains("DEFAULT_SQL = \"CURRENT_TIMESTAMP\"")
    );
    assert!(generated.code.contains("ON_UPDATE = \"CURRENT_TIMESTAMP\""));
    assert!(
        generated
            .code
            .contains("generated(VIRTUAL, \"CHAR_LENGTH(roles)\")")
    );
    assert!(generated.code.contains("FOREIGN_KEY(columns(account_id), references(Accounts, id), on_delete = \"CASCADE\", on_update = \"RESTRICT\")"));
    assert!(
        generated
            .code
            .contains("#[MySQLIndex(using = \"hash\", algorithm = \"inplace\", lock = \"none\")]")
    );
    assert!(generated.code.contains("struct ActiveAccounts {}"));
    assert!(generated.code.contains("#[derive(MySQLSchema)]"));

    let reparsed = parse_generated_ddl(&generated.code);
    let diff = compute_migration(&original, &reparsed).expect("equivalent DDL must diff");
    assert!(
        diff.statements.is_empty(),
        "round-trip changed MySQL DDL:\nsource:\n{}\nSQL: {:#?}",
        generated.code,
        diff.sql_statements
    );
}

#[test]
fn mysql_codegen_casing_keeps_sql_names_stable() {
    let original = rich_ddl();
    let generated = generate_rust_schema(
        &original,
        &CodegenOptions {
            field_casing: FieldCasing::Camel,
            ..CodegenOptions::default()
        },
    )
    .expect("rich DDL is representable");

    assert!(generated.code.contains("loginCount"));
    assert!(generated.code.contains("NAME = \"login_count\""));
    let reparsed = parse_generated_ddl(&generated.code);
    let diff = compute_migration(&original, &reparsed).expect("cased DDL must diff");
    assert!(
        diff.statements.is_empty(),
        "source:\n{}\nSQL: {:#?}",
        generated.code,
        diff.sql_statements
    );
}

#[test]
fn mysql_codegen_preserves_index_names_that_do_not_round_trip_through_rust() {
    let mut original = rich_ddl();
    original.indexes.list_mut()[0].name = "accounts_status_42".into();

    let generated = generate_rust_schema(&original, &CodegenOptions::default())
        .expect("explicit MySQL index names are representable");

    assert!(
        generated
            .code
            .contains("#[MySQLIndex(NAME = \"accounts_status_42\""),
        "generated source must preserve the physical index name:\n{}",
        generated.code
    );
    let reparsed = parse_generated_ddl(&generated.code);
    let diff = compute_migration(&original, &reparsed).expect("equivalent DDL must diff");
    assert!(
        diff.statements.is_empty(),
        "round-trip changed the MySQL index:\nsource:\n{}\nSQL: {:#?}",
        generated.code,
        diff.sql_statements
    );
}

#[test]
fn mysql_codegen_round_trips_expression_prefix_and_direction_key_parts() {
    let mut original = rich_ddl();
    let mut email = IndexColumn::column("email");
    email.length = Some(24);
    email.ascending = Some(false);
    let expression_sql = r#"concat(email, 'quoted "value"', 'path\segment')"#;
    let mut expression = IndexColumn::expression(expression_sql);
    expression.ascending = Some(true);
    let mut login_count = IndexColumn::column("login_count");
    login_count.ascending = Some(false);
    let mut index = Index::new(
        "accounts",
        "accounts_search_idx",
        vec![email, expression, login_count],
    );
    index.database = Some(DATABASE.into());
    original.indexes.push(index);

    let generated = generate_rust_schema(&original, &CodegenOptions::default())
        .expect("rich MySQL index key parts are representable");

    assert!(
        generated
            .code
            .contains("#[index(prefix = 24, desc)] Accounts::email")
    );
    assert!(generated.code.contains(
        r#"#[index(expr = "concat(email, 'quoted \"value\"', 'path\\segment')", asc)] Accounts::id"#
    ));
    assert!(
        generated
            .code
            .contains("#[index(desc)] Accounts::login_count")
    );
    let reparsed = parse_generated_ddl(&generated.code);
    let diff = compute_migration(&original, &reparsed).expect("equivalent DDL must diff");
    assert!(
        diff.statements.is_empty(),
        "round-trip changed rich MySQL index key parts:\nsource:\n{}\nSQL: {:#?}",
        generated.code,
        diff.sql_statements
    );
}

#[test]
fn mysql_codegen_warns_and_keeps_source_parseable_for_unrepresentable_metadata() {
    let mut ddl = rich_ddl();
    ddl.tables.list_mut()[0].options.push(TableOption {
        name: "ROW_FORMAT".into(),
        value: "DYNAMIC".into(),
    });
    ddl.indexes.list_mut()[0].comment = Some("not in MySQLIndex".into());
    ddl.views.list_mut()[0].definer = Some("root@localhost".into());
    ddl.columns.list_mut()[0].primary_key = false;
    ddl.columns.list_mut()[0].sql_type = "bigint unsigned".into();

    let generated = generate_rust_schema(&ddl, &CodegenOptions::default())
        .expect("remaining metadata is representable with warnings");
    assert!(
        generated
            .warnings
            .iter()
            .any(|warning| warning.contains("ROW_FORMAT"))
    );
    assert!(
        generated
            .warnings
            .iter()
            .any(|warning| warning.contains("index comment"))
    );
    assert!(
        generated
            .warnings
            .iter()
            .any(|warning| warning.contains("view definer"))
    );
    assert!(
        generated
            .warnings
            .iter()
            .any(|warning| warning.contains("Column.primary_key"))
    );
    assert!(
        generated
            .warnings
            .iter()
            .any(|warning| warning.contains("macro-canonical"))
    );
    assert!(generated.indexes.is_empty());

    let _ = parse_generated_ddl(&generated.code);
}

#[test]
fn mysql_codegen_canonicalizes_catalog_type_spellings_without_a_follow_up_change() {
    let mut ddl = rich_ddl();
    let login_count = ddl
        .columns
        .list_mut()
        .iter_mut()
        .find(|column| column.name == "login_count")
        .expect("login_count column");
    login_count.sql_type = "tinyint(1)".into();

    let generated = generate_rust_schema(&ddl, &CodegenOptions::default())
        .expect("TINYINT(1) has a lossless BOOLEAN representation");

    assert!(generated.code.contains("BOOLEAN"));
    assert!(
        !generated
            .code
            .contains("#[column(NAME = \"login_count\", TEXT")
    );
    let reparsed = parse_generated_ddl(&generated.code);
    let diff = compute_migration(&ddl, &reparsed).expect("canonical type spellings must diff");
    assert!(
        diff.statements.is_empty(),
        "canonical type spelling caused a follow-up migration: {:#?}",
        diff.sql_statements
    );
}

#[test]
fn mysql_codegen_rejects_valid_unrepresentable_column_types_instead_of_emitting_text() {
    let mut ddl = rich_ddl();
    ddl.columns.list_mut()[0].sql_type = "GEOMETRY".into();

    let error = generate_rust_schema(&ddl, &CodegenOptions::default())
        .expect_err("GEOMETRY has no MySQLTable representation");

    assert!(matches!(
        error,
        CodegenError::UnsupportedColumnType {
            ref table,
            ref column,
            ref sql_type,
        } if table == "accounts" && column == "id" && sql_type == "GEOMETRY"
    ));
    assert!(
        error
            .to_string()
            .contains("no lossless MySQLTable representation")
    );
}

#[test]
fn mysql_codegen_rejects_unrepresentable_enum_labels_instead_of_emitting_text() {
    let mut ddl = rich_ddl();
    let status = ddl
        .columns
        .list_mut()
        .iter_mut()
        .find(|column| column.name == "status")
        .expect("status column");
    status.sql_type = "ENUM('Draft', 'in progress')".into();
    status.inline_type = Some(InlineType::Enum(InlineEnum::new(["Draft", "in progress"])));

    let error = generate_rust_schema(&ddl, &CodegenOptions::default())
        .expect_err("a spaced enum label cannot become a Rust variant");

    assert!(matches!(
        error,
        CodegenError::UnsupportedEnumLabel {
            ref table,
            ref column,
            ref label,
        } if table == "accounts" && column == "status" && label == "in progress"
    ));
    assert!(error.to_string().contains("fieldless Rust enum variant"));
}

#[test]
fn mysql_view_definer_does_not_force_recreation_after_codegen() {
    let mut ddl = rich_ddl();
    ddl.views.list_mut()[0].definer = Some("`app_user`@`%`".into());

    let generated = generate_rust_schema(&ddl, &CodegenOptions::default())
        .expect("DEFINER is non-structural runtime metadata");
    let reparsed = parse_generated_ddl(&generated.code);
    let diff = compute_migration(&ddl, &reparsed).expect("view snapshots must diff");

    assert!(diff.statements.is_empty(), "{:#?}", diff.sql_statements);
}
