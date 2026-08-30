//! Driver-neutral acceptance tests for the MySQL procedural macros.
//!
//! These deliberately exercise generated metadata, models, and the shared
//! row-codec contract rather than a live connection. Concrete drivers retain
//! connection and row ownership while reusing this decoding policy.

use drizzle::core::{ColumnDialect, DrizzleTable, SQLSchemaImpl, TableDialect, ToSQL};
use drizzle::migrations::Schema as MigrationSchema;
use drizzle::mysql::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, MySQLEnum)]
enum AccountStatus {
    Draft,
    Published,
}

#[MySQLTable(
    DATABASE = "app_db",
    NAME = "accounts",
    ENGINE = "InnoDB",
    DEFAULT_CHARSET = "utf8mb4",
    COLLATE = "utf8mb4_0900_ai_ci"
)]
struct Accounts {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    #[column(VARCHAR(255))]
    email: String,
    #[column(ENUM)]
    status: AccountStatus,
    #[column(DEFAULT = 0)]
    login_count: u32,
    #[column(generated(STORED, "CHAR_LENGTH(email)"))]
    email_length_stored: u32,
    #[column(generated(VIRTUAL, "CHAR_LENGTH(email)"))]
    email_length_virtual: u32,
}

#[MySQLTable]
struct SerialIds {
    #[column(SERIAL)]
    id: u64,
    name: String,
}

#[MySQLTable]
struct DefaultMetadata {
    #[column(DEFAULT = 7)]
    database_value: i32,
    #[column(DEFAULT_FN = || 7)]
    application_value: i32,
}

#[MySQLTable]
struct NumericDeclarations {
    #[column(DECIMAL(20, 8))]
    decimal_value: String,
    #[column(DECIMAL_UNSIGNED(20, 8))]
    decimal_unsigned: String,
    #[column(FLOAT(10))]
    float_precision: f32,
    #[column(FLOAT_UNSIGNED(10, 2))]
    float_unsigned: f32,
    #[column(DOUBLE(10, 2))]
    double_value: f64,
    #[column(DOUBLE_UNSIGNED)]
    double_unsigned: f64,
    #[column(REAL(10, 2))]
    real_value: f64,
    #[column(REAL_UNSIGNED(10, 2))]
    real_unsigned: f64,
}

#[MySQLTable(SCHEMA = "audit_db", NAME = "account_events")]
struct AccountEvents {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    account_id: u64,
}

#[MySQLIndex(unique)]
struct AccountsEmailIdx(Accounts::email);

#[MySQLIndex(using = "btree", algorithm = "inplace", lock = "none")]
struct AccountsStatusIdx(Accounts::email);

#[MySQLIndex]
struct AccountsSearchIdx(
    #[index(prefix = 24, desc)] Accounts::email,
    #[index(expr = "lower(email)", asc)] Accounts::id,
);

#[derive(MySQLSchema)]
struct AppSchema {
    accounts: Accounts,
    accounts_email_idx: AccountsEmailIdx,
    accounts_search_idx: AccountsSearchIdx,
    accounts_status_idx: AccountsStatusIdx,
    account_events: AccountEvents,
}

#[derive(MySQLSchema)]
struct AccountsIndexSchema {
    accounts: Accounts,
    accounts_search_idx: AccountsSearchIdx,
}

#[MySQLView(
    DATABASE = "app_db",
    NAME = "account_emails",
    DEFINITION = "SELECT id, email FROM accounts",
    ALGORITHM = "merge",
    SQL_SECURITY = "invoker",
    CHECK_OPTION = "local"
)]
struct AccountEmails {
    id: u64,
    #[column(VARCHAR(255))]
    email: String,
}

#[MySQLView(
    DATABASE = "app_db",
    NAME = "typed_account_emails",
    ALGORITHM = "undefined",
    WITH_CHECK_OPTION,
    query(select(Accounts::id, Accounts::email), from(Accounts))
)]
struct TypedAccountEmails {
    id: u64,
    #[column(VARCHAR(255))]
    email: String,
}

#[MySQLView(DATABASE = "app_db", NAME = "external_accounts", EXISTING)]
struct ExternalAccounts {
    id: u64,
}

#[derive(MySQLSchema)]
struct ViewSchema {
    accounts: Accounts,
    account_emails: AccountEmails,
    typed_account_emails: TypedAccountEmails,
    external_accounts: ExternalAccounts,
}

#[MySQLView(
    DATABASE = "app_db",
    NAME = "a_dependent_account_ids",
    DEFINITION = "SELECT id FROM z_source_account_ids"
)]
struct DependentAccountIds {
    id: u64,
}

#[MySQLView(
    DATABASE = "app_db",
    NAME = "z_source_account_ids",
    DEFINITION = "SELECT id FROM accounts"
)]
struct SourceAccountIds {
    id: u64,
}

#[derive(MySQLSchema)]
struct DependentViewSchema {
    dependent: DependentAccountIds,
    source: SourceAccountIds,
    accounts: Accounts,
}

// Deliberately declare the dependent table first: MySQLSchema must still emit
// the same-database parent before its child.
#[MySQLTable(DATABASE = "billing", NAME = "qualified_children")]
struct QualifiedChild {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    #[column(REFERENCES = QualifiedParent::id, COMMENT = "links to the billing parent")]
    parent_id: u64,
}

#[MySQLTable(DATABASE = "billing", NAME = "qualified_parents")]
struct QualifiedParent {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
}

#[MySQLTable(DATABASE = "a_child_db", NAME = "cross_database_children")]
struct CrossDatabaseChild {
    #[column(PRIMARY)]
    id: u64,
    #[column(REFERENCES = CrossDatabaseParent::id)]
    parent_id: u64,
}

#[MySQLTable(DATABASE = "z_parent_db", NAME = "cross_database_parents")]
struct CrossDatabaseParent {
    #[column(PRIMARY)]
    id: u64,
}

#[derive(MySQLSchema)]
struct CrossDatabaseForeignKeySchema {
    child: CrossDatabaseChild,
    parent: CrossDatabaseParent,
}

#[MySQLTable]
struct ExpressionDefaults {
    #[column(PRIMARY)]
    id: u64,
    #[column(DEFAULT = "draft")]
    label: String,
    #[column(BLOB, DEFAULT = b"bytes")]
    payload: Vec<u8>,
    #[column(JSON, DEFAULT = "{}")]
    metadata: String,
    #[column(VARCHAR(255), DEFAULT = lower("DRAFT"))]
    normalized_label: String,
}

#[MySQLTable(DATABASE = "odd`db", NAME = "par`ents")]
struct EscapedParent {
    #[column(NAME = "i`d", PRIMARY)]
    id: u64,
    #[column(NAME = "pa`th", VARCHAR(255))]
    path: String,
}

#[MySQLTable(DATABASE = "odd`db", NAME = "chil`dren")]
struct EscapedChild {
    #[column(PRIMARY)]
    id: u64,
    #[column(REFERENCES = EscapedParent::id)]
    parent_id: u64,
}

#[MySQLView(
    NAME = "escaped_paths",
    query(
        select(EscapedParent::path),
        from(EscapedParent),
        filter(eq(EscapedParent::path, "C:\\tmp\\O'Brien"))
    )
)]
struct EscapedPaths {
    #[column(VARCHAR(255))]
    path: String,
}

#[cfg(feature = "uuid")]
#[MySQLTable]
struct InferredUuidStorage {
    id: uuid::Uuid,
}

#[cfg(feature = "arrayvec")]
#[MySQLTable]
struct InferredArrayVecStorage {
    name: arrayvec::ArrayString<32>,
    payload: arrayvec::ArrayVec<u8, 24>,
}

#[cfg(feature = "compact-str")]
#[MySQLTable]
struct InferredCompactStringStorage {
    value: compact_str::CompactString,
}

#[MySQLTable]
struct InferredFixedArrayStorage {
    bytes: [u8; 16],
    chars: [char; 8],
}

#[MySQLTable]
struct InferredZeroLengthArrayStorage {
    bytes: [u8; 0],
    chars: [char; 0],
}

#[cfg(feature = "arrayvec")]
#[MySQLTable]
struct InferredZeroCapacityArrayVecStorage {
    name: arrayvec::ArrayString<0>,
    payload: arrayvec::ArrayVec<u8, 0>,
}

#[cfg(feature = "bytes")]
#[MySQLTable]
struct InferredBytesStorage {
    bytes: bytes::Bytes,
    bytes_mut: bytes::BytesMut,
}

#[cfg(feature = "smallvec-types")]
#[MySQLTable]
struct InferredSmallVecStorage {
    value: smallvec::SmallVec<[u8; 16]>,
}

#[MySQLIndex]
struct EscapedParentIdIdx(EscapedParent::id);

#[derive(MySQLSchema)]
struct QualifiedForeignKeySchema {
    child: QualifiedChild,
    parent: QualifiedParent,
}

#[derive(MySQLFromRow)]
#[from(Accounts)]
struct AccountRow {
    id: u64,
    #[column(Accounts::email)]
    email: String,
}

fn assert_mysql_selector<T>()
where
    for<'a> T: ToSQL<'a, drizzle::mysql::MySQLValue<'a>>,
{
}

fn assert_mysql_selector_value<T>(_: T)
where
    for<'a> T: ToSQL<'a, drizzle::mysql::MySQLValue<'a>>,
{
}

fn assert_mysql_expr<T>()
where
    for<'a> T: drizzle::core::expr::Expr<'a, drizzle::mysql::MySQLValue<'a>>,
{
}

fn assert_mysql_bind_type<T: drizzle::core::ValueTypeForDialect<drizzle::mysql::MySQLDialect>>() {}

fn assert_mysql_bind_type_is<T, SQLType>()
where
    T: drizzle::core::ValueTypeForDialect<drizzle::mysql::MySQLDialect, SQLType = SQLType>,
    SQLType: drizzle::core::types::DataType,
{
}

fn assert_mysql_select_model<T>()
where
    T: drizzle::core::HasSelectModel<SelectModel = SelectAccounts>,
{
}

#[test]
fn table_metadata_preserves_mysql_database_unsigned_and_generated_details() {
    let table = &<Accounts as DrizzleTable>::TABLE_REF;

    assert_eq!(table.name, "accounts");
    assert_eq!(table.schema, Some("app_db"));
    assert_eq!(table.qualified_name, "app_db.accounts");
    assert!(matches!(
        table.dialect,
        TableDialect::MySQL {
            engine: Some("InnoDB"),
            charset: Some("utf8mb4"),
            collate: Some("utf8mb4_0900_ai_ci"),
            ..
        }
    ));

    let id = &table.columns[0];
    assert_eq!(id.name, "id");
    assert_eq!(id.sql_type, "BIGINT UNSIGNED");
    assert!(matches!(
        id.dialect,
        ColumnDialect::MySQL {
            auto_increment: true,
            ..
        }
    ));

    let login_count = &table.columns[3];
    assert_eq!(login_count.sql_type, "INT UNSIGNED");

    assert!(matches!(
        table.columns[4].dialect,
        ColumnDialect::MySQL {
            generated_expression: Some("CHAR_LENGTH(email)"),
            generated_stored: true,
            ..
        }
    ));
    assert!(matches!(
        table.columns[5].dialect,
        ColumnDialect::MySQL {
            generated_expression: Some("CHAR_LENGTH(email)"),
            generated_stored: false,
            ..
        }
    ));

    let alias = &<AccountEvents as DrizzleTable>::TABLE_REF;
    assert_eq!(alias.schema, Some("audit_db"));
    assert_eq!(alias.qualified_name, "audit_db.account_events");

    let sql = Accounts::create_table_sql();
    assert!(sql.contains("CREATE TABLE `app_db`.`accounts`"));
    assert!(
        sql.ends_with(") ENGINE=InnoDB DEFAULT CHARACTER SET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;")
    );
    assert!(sql.contains("`id` BIGINT UNSIGNED PRIMARY KEY NOT NULL AUTO_INCREMENT"));
    assert!(sql.contains("`login_count` INT UNSIGNED NOT NULL DEFAULT 0"));
    assert!(sql.contains("`status` ENUM('Draft', 'Published') NOT NULL"));
    assert!(sql.contains(
        "`email_length_stored` INT UNSIGNED GENERATED ALWAYS AS (CHAR_LENGTH(email)) STORED NOT NULL"
    ));
    assert!(sql.contains(
        "`email_length_virtual` INT UNSIGNED GENERATED ALWAYS AS (CHAR_LENGTH(email)) VIRTUAL NOT NULL"
    ));
    assert_eq!(Accounts::new().to_sql().sql(), "`app_db`.`accounts`");
}

#[test]
fn mysql_views_use_the_standard_typed_table_api_and_render_mysql_ddl() {
    assert_mysql_selector::<AccountEmails>();
    assert_mysql_expr::<AccountEmailsEmail>();

    let sql = AccountEmails::create_view_sql();
    assert_eq!(
        sql,
        "CREATE ALGORITHM=MERGE SQL SECURITY INVOKER VIEW `app_db`.`account_emails` AS SELECT id, email FROM accounts WITH LOCAL CHECK OPTION;"
    );
    assert_eq!(
        TypedAccountEmails::ddl_sql(),
        "CREATE ALGORITHM=UNDEFINED VIEW `app_db`.`typed_account_emails` AS SELECT `app_db`.`accounts`.`id` AS `id`, `app_db`.`accounts`.`email` AS `email` FROM `app_db`.`accounts` WITH CASCADED CHECK OPTION;"
    );

    let selected = drizzle::mysql::builder::QueryBuilder::new::<ViewSchema>()
        .select(())
        .from(AccountEmails::default())
        .r#where(drizzle::core::expr::eq(AccountEmails::id, 1_u64))
        .to_sql()
        .sql();
    assert!(selected.contains("`app_db`.`account_emails`"));

    assert_eq!(
        EscapedPaths::ddl_sql(),
        "CREATE VIEW `escaped_paths` AS SELECT `odd``db`.`par``ents`.`pa``th` AS `path` FROM `odd``db`.`par``ents` WHERE `odd``db`.`par``ents`.`pa``th` = 'C:\\\\tmp\\\\O''Brien';"
    );
}

#[test]
fn mysql_schema_emits_views_last_and_preserves_typed_view_metadata() {
    let statements = ViewSchema::new()
        .create_statements()
        .expect("view schema should be valid")
        .collect::<Vec<_>>();
    assert_eq!(statements.len(), 3);
    assert!(statements[0].starts_with("CREATE TABLE"));
    assert!(statements[1].starts_with("CREATE ALGORITHM=MERGE"));
    assert!(statements[2].starts_with("CREATE ALGORITHM=UNDEFINED"));

    let drizzle::migrations::Snapshot::MySQL(snapshot) = ViewSchema::new().to_snapshot() else {
        panic!("expected MySQL snapshot");
    };
    let view = snapshot
        .ddl
        .iter()
        .find_map(|entity| match entity {
            drizzle::migrations::mysql::MySQLEntity::View(view)
                if view.name.as_ref() == "account_emails" =>
            {
                Some(view)
            }
            _ => None,
        })
        .expect("account_emails view metadata");
    assert_eq!(view.database.as_deref(), Some("app_db"));
    assert_eq!(
        view.definition.as_deref(),
        Some("SELECT id, email FROM accounts")
    );
    assert_eq!(
        view.algorithm,
        Some(drizzle::ddl::mysql::ddl::ViewAlgorithm::Merge)
    );
    assert_eq!(view.definer, None);
    assert_eq!(
        view.sql_security,
        Some(drizzle::ddl::mysql::ddl::ViewSqlSecurity::Invoker)
    );
    assert_eq!(
        view.check_option,
        Some(drizzle::ddl::mysql::ddl::ViewCheckOption::Local)
    );
    assert_eq!(view.charset, None);
    assert_eq!(view.collation, None);

    let existing = snapshot
        .ddl
        .iter()
        .find_map(|entity| match entity {
            drizzle::migrations::mysql::MySQLEntity::View(view)
                if view.name.as_ref() == "external_accounts" =>
            {
                Some(view)
            }
            _ => None,
        })
        .expect("external view metadata");
    assert!(existing.is_existing);
    assert_eq!(existing.definition, None);
}

#[test]
fn mysql_schema_orders_a_source_view_before_its_dependent_view() {
    let statements = DependentViewSchema::new()
        .create_statements()
        .expect("dependent view schema should be valid")
        .collect::<Vec<_>>();

    let source_position = statements
        .iter()
        .position(|statement| statement.contains("VIEW `app_db`.`z_source_account_ids`"))
        .expect("source view CREATE statement");
    let dependent_position = statements
        .iter()
        .position(|statement| statement.contains("VIEW `app_db`.`a_dependent_account_ids`"))
        .expect("dependent view CREATE statement");

    assert!(
        source_position < dependent_position,
        "source view must be created before its dependent: {statements:?}"
    );
}

#[cfg(feature = "query")]
#[test]
fn relational_metadata_preserves_mysql_database_boundaries() {
    use drizzle::core::query::QueryTable;

    assert_eq!(Accounts::TABLE.schema, Some("app_db"));
    assert_eq!(Accounts::TABLE.name, "accounts");
    assert_eq!(QualifiedChild::TABLE.schema, Some("billing"));
    assert_eq!(QualifiedParent::TABLE.schema, Some("billing"));
    assert_eq!(CrossDatabaseChild::TABLE.schema, Some("a_child_db"));
    assert_eq!(CrossDatabaseParent::TABLE.schema, Some("z_parent_db"));
}

#[test]
fn serial_expands_to_mysql_bigint_unsigned_alias() {
    assert!(SerialIds::ddl_sql().contains("`id` BIGINT UNSIGNED UNIQUE NOT NULL AUTO_INCREMENT"));
    let _insert = InsertSerialIds::new("serial");
}

#[test]
fn generated_models_have_the_expected_mysql_surface() {
    use drizzle::core::SQLModel as _;

    assert_mysql_select_model::<Accounts>();
    assert_eq!(<Accounts as drizzle::core::HasSelectModel>::COLUMN_COUNT, 6);

    let insert =
        InsertAccounts::new("hello@example.test", AccountStatus::Draft).with_login_count(7_u32);
    assert_eq!(
        insert
            .columns()
            .iter()
            .map(|column| column.name)
            .collect::<Vec<_>>(),
        ["email", "status", "login_count"]
    );
    let insert_values = insert.values();
    assert_eq!(insert_values.sql(), "?, ?, ?");
    assert_eq!(insert_values.params().count(), 3);
    let owned = insert.into_owned();
    assert_eq!(owned.values().sql(), "?, ?, ?");
    assert_eq!(owned.values().params().count(), 3);

    let update = UpdateAccounts::default().with_email("renamed@example.test");
    assert_eq!(update.to_sql().sql(), "`email` = ?");
    assert_eq!(update.to_sql().params().count(), 1);

    let select = SelectAccounts {
        id: 1_u64,
        email: "hello@example.test".to_owned(),
        status: AccountStatus::Published,
        login_count: 7_u32,
        email_length_stored: 18_u32,
        email_length_virtual: 18_u32,
    };
    let SelectAccounts {
        id,
        login_count,
        email_length_stored,
        email_length_virtual,
        ..
    } = select;
    let _: u64 = id;
    let _: u32 = login_count;
    let _: u32 = email_length_stored;
    let _: u32 = email_length_virtual;
}

#[test]
fn enum_index_schema_and_from_row_are_generated_for_mysql() {
    use drizzle::core::FromDrizzleRow as _;
    use drizzle::mysql::OwnedMySQLValue;
    use drizzle::mysql::driver::MySQLRow;
    use drizzle::mysql::traits::MySQLEnum as _;

    assert_eq!(AccountStatus::VARIANTS, &["Draft", "Published"]);
    assert_eq!(AccountStatus::Draft.variant_name(), "Draft");
    assert_eq!(
        "Published"
            .parse::<AccountStatus>()
            .expect("known enum label"),
        AccountStatus::Published
    );
    assert!("missing".parse::<AccountStatus>().is_err());
    assert_eq!(AccountStatus::SQL_TYPE, "ENUM('Draft', 'Published')");
    assert_mysql_expr::<AccountStatus>();
    assert_mysql_bind_type::<AccountStatus>();
    assert_mysql_bind_type::<&AccountStatus>();

    let schema = AppSchema::new();
    assert_eq!(schema.table_refs().len(), 2);
    assert_eq!(<Accounts as DrizzleTable>::NAME, "accounts");
    assert_eq!(<AccountEvents as DrizzleTable>::NAME, "account_events");
    let _ = schema.accounts_email_idx;
    let _ = schema.accounts_status_idx;

    let index_sql = AccountsEmailIdx::new().to_sql().sql();
    assert_eq!(
        index_sql,
        "CREATE UNIQUE INDEX `accounts_email_idx` ON `app_db`.`accounts`(`email`);"
    );
    assert_eq!(
        AccountsStatusIdx::DDL_SQL,
        "CREATE INDEX `accounts_status_idx` USING BTREE ON `app_db`.`accounts`(`email`) ALGORITHM=INPLACE LOCK=NONE;"
    );
    assert_eq!(AccountsStatusIdx::METHOD, Some(MySQLIndexMethod::BTree));
    assert_eq!(
        AccountsStatusIdx::ALGORITHM,
        Some(MySQLIndexAlgorithm::Inplace)
    );
    assert_eq!(AccountsStatusIdx::LOCK, Some(MySQLIndexLock::None));
    assert_eq!(
        AccountsSearchIdx::DDL_SQL,
        "CREATE INDEX `accounts_search_idx` ON `app_db`.`accounts`(`email`(24) DESC, (lower(email)) ASC);"
    );
    assert_eq!(
        AccountsSearchIdx::KEY_PARTS,
        &[
            IndexKeyPart::Column {
                name: "email",
                length: Some(24),
                order: Some(IndexOrder::Desc),
            },
            IndexKeyPart::Expression {
                sql: "lower(email)",
                order: Some(IndexOrder::Asc),
            },
        ]
    );

    let snapshot = AccountsIndexSchema::new().to_snapshot();
    let drizzle::migrations::Snapshot::MySQL(snapshot) = snapshot else {
        panic!("MySQL schema produced another snapshot dialect");
    };
    let ddl = drizzle::migrations::mysql::MySQLDDL::try_from_entities(snapshot.ddl)
        .expect("generated MySQL index metadata is valid");
    let index = ddl
        .indexes
        .one(Some("app_db"), "accounts", "accounts_search_idx")
        .expect("rich index is present in the generated snapshot");
    assert_eq!(index.columns.len(), 2);
    assert_eq!(index.columns[0].expression, "email");
    assert!(!index.columns[0].is_expression);
    assert_eq!(index.columns[0].length, Some(24));
    assert_eq!(index.columns[0].ascending, Some(false));
    assert_eq!(index.columns[1].expression, "lower(email)");
    assert!(index.columns[1].is_expression);
    assert_eq!(index.columns[1].length, None);
    assert_eq!(index.columns[1].ascending, Some(true));

    assert_mysql_selector::<AccountRow>();
    assert_mysql_selector_value(AccountRow::Select);

    let row = AccountRow {
        id: 42,
        email: "row@example.test".to_owned(),
    };
    assert_eq!(row.id, 42);
    assert_eq!(row.email, "row@example.test");

    let raw = [
        OwnedMySQLValue::UInt(42),
        OwnedMySQLValue::Bytes(b"decoded@example.test".to_vec()),
    ];
    let decoded = AccountRow::from_row(&MySQLRow::new(raw.as_slice())).unwrap();
    assert_eq!(decoded.id, 42);
    assert_eq!(decoded.email, "decoded@example.test");

    let with_prefix = [
        OwnedMySQLValue::UInt(9),
        OwnedMySQLValue::UInt(42),
        OwnedMySQLValue::Bytes(b"offset@example.test".to_vec()),
    ];
    let (prefix, decoded) =
        <(u8, AccountRow)>::from_row(&MySQLRow::new(with_prefix.as_slice())).unwrap();
    assert_eq!(prefix, 9);
    assert_eq!(decoded.email, "offset@example.test");

    let selected = [
        OwnedMySQLValue::UInt(7),
        OwnedMySQLValue::Bytes(b"selected@example.test".to_vec()),
        OwnedMySQLValue::Bytes(b"Published".to_vec()),
        OwnedMySQLValue::UInt(3),
        OwnedMySQLValue::UInt(21),
        OwnedMySQLValue::UInt(21),
    ];
    let selected = SelectAccounts::from_row(&MySQLRow::new(selected.as_slice())).unwrap();
    assert_eq!(selected.id, 7);
    assert_eq!(selected.status, AccountStatus::Published);
    assert_eq!(selected.login_count, 3);

    let absent: [OwnedMySQLValue; 6] = core::array::from_fn(|_| OwnedMySQLValue::Null);
    assert!(
        Option::<SelectAccounts>::from_row(&MySQLRow::new(absent.as_slice()))
            .unwrap()
            .is_none()
    );
}

#[test]
fn schema_orders_same_database_qualified_foreign_key_parents_before_children() {
    let schema = QualifiedForeignKeySchema::new();
    let statements: Vec<_> = schema
        .create_statements()
        .expect("qualified foreign-key schema is valid")
        .collect();

    assert_eq!(statements.len(), 2);
    let parent_position = statements
        .iter()
        .position(|statement| statement.contains("CREATE TABLE `billing`.`qualified_parents`"))
        .expect("parent CREATE statement");
    let child_position = statements
        .iter()
        .position(|statement| statement.contains("CREATE TABLE `billing`.`qualified_children`"))
        .expect("child CREATE statement");

    assert!(
        parent_position < child_position,
        "qualified parent must be created before its child: {statements:?}"
    );
    assert!(
        statements[child_position]
            .contains("FOREIGN KEY (`parent_id`) REFERENCES `billing`.`qualified_parents` (`id`)")
    );
}

#[test]
fn mysql_schema_snapshot_retains_column_comment_in_canonical_order() {
    let snapshot = QualifiedForeignKeySchema::new().to_snapshot();
    let drizzle::migrations::Snapshot::MySQL(snapshot) = snapshot else {
        panic!("expected MySQL snapshot");
    };

    let parent_id = snapshot
        .ddl
        .iter()
        .find_map(|entity| match entity {
            drizzle::migrations::mysql::MySQLEntity::Column(column)
                if column.table.as_ref() == "qualified_children"
                    && column.name.as_ref() == "parent_id" =>
            {
                Some(column)
            }
            _ => None,
        })
        .expect("qualified_children.parent_id column");
    assert_eq!(
        parent_id.comment.as_deref(),
        Some("links to the billing parent")
    );

    let category = |entity: &drizzle::migrations::mysql::MySQLEntity| match entity {
        drizzle::migrations::mysql::MySQLEntity::Table(_) => 0,
        drizzle::migrations::mysql::MySQLEntity::Column(_) => 1,
        drizzle::migrations::mysql::MySQLEntity::PrimaryKey(_) => 2,
        drizzle::migrations::mysql::MySQLEntity::UniqueConstraint(_) => 3,
        drizzle::migrations::mysql::MySQLEntity::Index(_) => 4,
        drizzle::migrations::mysql::MySQLEntity::ForeignKey(_) => 5,
        drizzle::migrations::mysql::MySQLEntity::CheckConstraint(_) => 6,
        drizzle::migrations::mysql::MySQLEntity::View(_) => 7,
    };
    assert!(
        snapshot
            .ddl
            .windows(2)
            .all(|pair| category(&pair[0]) <= category(&pair[1])),
        "MySQL macro snapshot entities must use canonical category order"
    );
}

#[test]
fn schema_orders_cross_database_foreign_key_parents_before_children() {
    let statements: Vec<_> = CrossDatabaseForeignKeySchema::new()
        .create_statements()
        .expect("cross-database foreign-key schema is valid")
        .collect();

    assert!(statements[0].contains("CREATE TABLE `z_parent_db`.`cross_database_parents`"));
    assert!(statements[1].contains("CREATE TABLE `a_child_db`.`cross_database_children`"));
    assert!(statements[1].contains("REFERENCES `z_parent_db`.`cross_database_parents` (`id`)"));
}

#[test]
fn text_blob_and_json_defaults_are_rendered_as_mysql_expressions() {
    let sql = ExpressionDefaults::create_table_sql();
    assert!(sql.contains("`label` TEXT NOT NULL DEFAULT ('draft')"));
    assert!(sql.contains("`payload` BLOB NOT NULL DEFAULT (X'6279746573')"));
    assert!(sql.contains("`metadata` JSON NOT NULL DEFAULT ('{}')"));
    assert!(sql.contains("`normalized_label` VARCHAR(255) NOT NULL DEFAULT (lower('DRAFT'))"));
}

#[test]
fn numeric_metadata_preserves_arguments_unsigned_and_real() {
    let table = &<NumericDeclarations as DrizzleTable>::TABLE_REF;
    let sql_types = table
        .columns
        .iter()
        .map(|column| column.sql_type)
        .collect::<Vec<_>>();
    assert_eq!(
        sql_types,
        [
            "DECIMAL(20, 8)",
            "DECIMAL(20, 8) UNSIGNED",
            "FLOAT(10)",
            "FLOAT(10, 2) UNSIGNED",
            "DOUBLE(10, 2)",
            "DOUBLE UNSIGNED",
            "REAL(10, 2)",
            "REAL(10, 2) UNSIGNED",
        ]
    );

    let sql = NumericDeclarations::create_table_sql();
    assert!(sql.contains("`decimal_unsigned` DECIMAL(20, 8) UNSIGNED NOT NULL"));
    assert!(sql.contains("`float_unsigned` FLOAT(10, 2) UNSIGNED NOT NULL"));
    assert!(sql.contains("`double_unsigned` DOUBLE UNSIGNED NOT NULL"));
    assert!(sql.contains("`real_value` REAL(10, 2) NOT NULL"));
    assert!(sql.contains("`real_unsigned` REAL(10, 2) UNSIGNED NOT NULL"));
}

#[test]
fn column_metadata_distinguishes_database_and_application_defaults() {
    let columns = <DefaultMetadata as DrizzleTable>::TABLE_REF.columns;
    assert!(columns[0].has_default());
    assert!(!columns[1].has_default());
}

#[test]
fn referenced_and_indexed_identifiers_escape_embedded_backticks() {
    assert_eq!(EscapedParent::new().to_sql().sql(), "`odd``db`.`par``ents`");
    assert!(EscapedChild::create_table_sql().contains("REFERENCES `odd``db`.`par``ents` (`i``d`)"));
    assert_eq!(
        EscapedParentIdIdx::DDL_SQL,
        "CREATE INDEX `escaped_parent_id_idx` ON `odd``db`.`par``ents`(`i``d`);"
    );
}

#[test]
fn fixed_array_capacities_are_preserved_in_inferred_mysql_types() {
    let sql = InferredFixedArrayStorage::create_table_sql();
    assert!(sql.contains("`bytes` BINARY(16) NOT NULL"));
    assert!(sql.contains("`chars` CHAR(8) NOT NULL"));

    let insert = InsertInferredFixedArrayStorage::new([1_u8; 16], ['x'; 8]);
    let _owned = insert.into_owned();
    assert_mysql_expr::<[u8; 16]>();
    assert_mysql_expr::<[char; 8]>();
    assert_mysql_bind_type_is::<[u8; 16], drizzle::mysql::types::Binary>();
    assert_mysql_bind_type_is::<[char; 8], drizzle::mysql::types::Char>();

    let condition = drizzle::core::expr::eq(InferredFixedArrayStorage::new().chars, ['x'; 8]);
    let condition_sql = condition.to_sql();
    assert_eq!(
        condition_sql.sql(),
        "`inferred_fixed_array_storage`.`chars` = ?"
    );
    assert_eq!(condition_sql.params().count(), 1);
    assert_eq!(
        condition_sql
            .params()
            .next()
            .expect("character-array bind parameter")
            .as_bytes(),
        Some(b"xxxxxxxx".as_slice())
    );
}

#[test]
fn zero_length_fixed_arrays_preserve_valid_mysql_lengths() {
    let sql = InferredZeroLengthArrayStorage::create_table_sql();
    assert!(sql.contains("`bytes` BINARY(0) NOT NULL"));
    assert!(sql.contains("`chars` CHAR(0) NOT NULL"));
}

#[cfg(feature = "uuid")]
#[test]
fn uuid_inference_uses_exact_sixteen_byte_storage() {
    assert!(InferredUuidStorage::create_table_sql().contains("`id` BINARY(16) NOT NULL"));
    let insert = InsertInferredUuidStorage::new(uuid::Uuid::nil());
    let _owned = insert.into_owned();
}

#[cfg(feature = "arrayvec")]
#[test]
fn bounded_arrayvec_types_infer_bounded_mysql_columns() {
    let sql = InferredArrayVecStorage::create_table_sql();
    assert!(sql.contains("`name` VARCHAR(32) NOT NULL"));
    assert!(sql.contains("`payload` VARBINARY(24) NOT NULL"));
    assert_mysql_expr::<arrayvec::ArrayString<32>>();
    assert_mysql_expr::<arrayvec::ArrayVec<u8, 24>>();
    assert_mysql_bind_type_is::<arrayvec::ArrayString<32>, drizzle::mysql::types::Varchar>();
    assert_mysql_bind_type_is::<arrayvec::ArrayVec<u8, 24>, drizzle::mysql::types::Varbinary>();
}

#[cfg(feature = "arrayvec")]
#[test]
fn zero_capacity_arrayvec_types_preserve_valid_mysql_lengths() {
    let sql = InferredZeroCapacityArrayVecStorage::create_table_sql();
    assert!(sql.contains("`name` VARCHAR(0) NOT NULL"));
    assert!(sql.contains("`payload` VARBINARY(0) NOT NULL"));
}

#[cfg(feature = "compact-str")]
#[test]
fn compact_strings_infer_unbounded_text_storage() {
    assert!(InferredCompactStringStorage::create_table_sql().contains("`value` TEXT NOT NULL"));
    assert_mysql_expr::<compact_str::CompactString>();
}

#[cfg(feature = "bytes")]
#[test]
fn growable_bytes_types_infer_blob_storage() {
    let sql = InferredBytesStorage::create_table_sql();
    assert!(sql.contains("`bytes` BLOB NOT NULL"));
    assert!(sql.contains("`bytes_mut` BLOB NOT NULL"));
    assert_mysql_expr::<bytes::Bytes>();
    assert_mysql_expr::<bytes::BytesMut>();
}

#[cfg(feature = "smallvec-types")]
#[test]
fn spillable_smallvec_types_infer_blob_storage() {
    assert!(InferredSmallVecStorage::create_table_sql().contains("`value` BLOB NOT NULL"));
    assert_mysql_expr::<smallvec::SmallVec<[u8; 16]>>();
}
