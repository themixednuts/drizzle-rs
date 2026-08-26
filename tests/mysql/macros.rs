//! Driver-neutral acceptance tests for the MySQL procedural macros.
//!
//! These deliberately exercise generated metadata and models rather than a
//! live connection. Wire-level decoding belongs with the concrete MySQL
//! drivers; the macro contract is still useful and testable without one.

use drizzle::core::{ColumnDialect, DrizzleTable, SQLSchemaImpl, TableDialect, ToSQL};
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

#[derive(MySQLSchema)]
struct AppSchema {
    accounts: Accounts,
    accounts_email_idx: AccountsEmailIdx,
    accounts_status_idx: AccountsStatusIdx,
    account_events: AccountEvents,
}

// Deliberately declare the dependent table first: MySQLSchema must still emit
// the same-database parent before its child.
#[MySQLTable(DATABASE = "billing", NAME = "qualified_children")]
struct QualifiedChild {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    #[column(REFERENCES = QualifiedParent::id)]
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
}

#[MySQLTable(DATABASE = "odd`db", NAME = "par`ents")]
struct EscapedParent {
    #[column(NAME = "i`d", PRIMARY)]
    id: u64,
}

#[MySQLTable(DATABASE = "odd`db", NAME = "chil`dren")]
struct EscapedChild {
    #[column(PRIMARY)]
    id: u64,
    #[column(REFERENCES = EscapedParent::id)]
    parent_id: u64,
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
    use drizzle::mysql::traits::MySQLEnum as _;

    assert_eq!(AccountStatus::VARIANTS, &["Draft", "Published"]);
    assert_eq!(AccountStatus::Draft.variant_name(), "Draft");
    assert_eq!(
        AccountStatus::try_from_str("Published").expect("known enum label"),
        AccountStatus::Published
    );
    assert!(AccountStatus::try_from_str("missing").is_err());
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

    assert_mysql_selector::<AccountRow>();
    assert_mysql_selector_value(AccountRow::Select);

    let row = AccountRow {
        id: 42,
        email: "row@example.test".to_owned(),
    };
    assert_eq!(row.id, 42);
    assert_eq!(row.email, "row@example.test");
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
