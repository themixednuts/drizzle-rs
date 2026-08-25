#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;

// Test enums with different representations
#[derive(SQLiteEnum, PartialEq, Clone, Default, Debug)]
pub enum UserRole {
    #[default]
    Guest,
    Member,
    Admin,
}

#[derive(SQLiteEnum, Default, Debug, Clone, PartialEq)]
pub enum AccountStatus {
    Suspended = -1,
    #[default]
    Inactive = 3,
    Active,
}

#[derive(SQLiteEnum, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
enum SharedJobStatus {
    #[default]
    Queued = 0,
    Complete = 1,
}

// Table with enum fields using different column types
#[SQLiteTable]
struct UserAccount {
    #[column(PRIMARY, AUTOINCREMENT)]
    id: i64,
    name: String,
    #[column(ENUM)]
    role: UserRole,
    #[column(integer, ENUM)]
    status: AccountStatus,
}

#[SQLiteTable]
struct PrimaryJob {
    #[column(PRIMARY, AUTOINCREMENT)]
    id: i64,
    status: SharedJobStatus,
}

#[SQLiteTable]
struct SecondaryJob {
    #[column(PRIMARY, AUTOINCREMENT)]
    id: i64,
    status: SharedJobStatus,
}

#[SQLiteTable]
struct NullableJob {
    #[column(PRIMARY, AUTOINCREMENT)]
    id: i64,
    #[column(integer, ENUM)]
    status: Option<SharedJobStatus>,
}

#[derive(SQLiteSchema)]
pub struct Schema {
    user_account: UserAccount,
    primary_job: PrimaryJob,
    secondary_job: SecondaryJob,
    nullable_job: NullableJob,
}

#[test]
fn test_enum() {
    // Test basic enum functionality works
    let role = UserRole::Admin;
    let status = AccountStatus::Active;

    // Test Display
    assert_eq!(format!("{}", role), "Admin");
    assert_eq!(format!("{}", status), "Active");

    // Test FromStr
    assert_eq!("Member".parse::<UserRole>().unwrap(), UserRole::Member);
    assert_eq!(
        "Suspended".parse::<AccountStatus>().unwrap(),
        AccountStatus::Suspended
    );
    assert!(SharedJobStatus::decode(SQLiteValueRef::Text("Complete")).is_err());
    assert!(UserRole::decode(SQLiteValueRef::Integer(0)).is_err());
}

#[cfg(feature = "query")]
#[test]
fn enum_query_codec_rejects_wrong_storage_class() {
    let integer_as_text = drizzle::core::serde_json::json!({
        "$drizzle_storage": "text",
        "$drizzle_value": "Complete",
    });
    let text_as_integer = drizzle::core::serde_json::json!({
        "$drizzle_storage": "integer",
        "$drizzle_value": 0,
    });

    assert!(SharedJobStatus::decode_json(&integer_as_text).is_err());
    assert!(UserRole::decode_json(&text_as_integer).is_err());
}

#[test]
fn test_table_generation() {
    // Just test that the table compiles and has the expected structure
    let _table = UserAccount::new();

    // Test that we can create insert and update models
    let insert_model = InsertUserAccount::new("test", UserRole::Member, AccountStatus::Suspended);
    let update_model = UpdateUserAccount::default();

    // Test convenience methods work with enums
    let _insert_with_role = insert_model.with_role(UserRole::Member);
    let _update_with_status = update_model.with_status(AccountStatus::Suspended);

    // Basic smoke test - if this compiles, the From implementations were generated correctly
    let table_ref = &<UserAccount as drizzle::core::DrizzleTable>::TABLE_REF;
    assert!(!table_ref.columns.is_empty());
}

#[test]
fn repr_enum_without_column_marker_uses_integer_ddl_in_multiple_tables() {
    assert!(
        PrimaryJob::create_table_sql().contains("`status` INTEGER NOT NULL"),
        "primary table must use the enum's INTEGER storage authority"
    );
    assert!(
        SecondaryJob::create_table_sql().contains("`status` INTEGER NOT NULL"),
        "secondary table must use the same enum's INTEGER storage authority"
    );
    assert_eq!(
        <PrimaryJob as drizzle::core::DrizzleTable>::TABLE_REF.columns[1].sql_type,
        "INTEGER"
    );
}

#[cfg(feature = "rusqlite")]
#[test]
fn repr_enum_rusqlite_parameter_binds_as_integer() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    connection
        .execute_batch("CREATE TABLE jobs (status INTEGER NOT NULL) STRICT")
        .unwrap();
    connection
        .execute(
            "INSERT INTO jobs (status) VALUES (?1)",
            rusqlite::params![SharedJobStatus::Complete],
        )
        .unwrap();
    let storage_class: String = connection
        .query_row("SELECT typeof(status) FROM jobs", [], |row| row.get(0))
        .unwrap();

    assert_eq!(storage_class, "integer");
    assert!(
        connection
            .query_row("SELECT 'Complete'", [], |row| {
                row.get::<_, SharedJobStatus>(0)
            })
            .is_err()
    );
}

#[cfg(feature = "libsql")]
#[tokio::test]
async fn nullable_libsql_enum_rejects_invalid_non_null_value() {
    let database = libsql::Builder::new_local(":memory:")
        .build()
        .await
        .unwrap();
    let connection = database.connect().unwrap();
    connection
        .execute(
            "CREATE TABLE nullable_job (id INTEGER PRIMARY KEY, status INTEGER)",
            (),
        )
        .await
        .unwrap();
    connection
        .execute("INSERT INTO nullable_job VALUES (1, 99)", ())
        .await
        .unwrap();
    connection
        .execute("INSERT INTO nullable_job VALUES (2, NULL)", ())
        .await
        .unwrap();
    let mut rows = connection
        .query("SELECT id, status FROM nullable_job WHERE id = 1", ())
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();

    let error = SelectNullableJob::try_from(&row).expect_err("99 is not a valid enum value");
    assert!(
        error.to_string().contains("99"),
        "unexpected error: {error}"
    );

    let mut rows = connection
        .query("SELECT id, status FROM nullable_job WHERE id = 2", ())
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let decoded = SelectNullableJob::try_from(&row).unwrap();
    assert_eq!(decoded.status, None);
}

// Enum types work directly in FromRow for all drivers (rusqlite, libsql, turso)
// - rusqlite: Uses FromSql trait generated by SQLiteEnum
// - libsql/turso: Uses TryFrom<i64> and TryFrom<&str> generated by SQLiteEnum
#[allow(dead_code)]
#[derive(Debug, SQLiteFromRow)]
struct UserAccountResult {
    id: i64,
    name: String,
    role: UserRole,        // Direct enum type - works with TEXT storage
    status: AccountStatus, // Direct enum type - works with INTEGER storage
}

#[drizzle::test]
fn test_enum_database_roundtrip(db: &mut TestDb<Schema>) {
    let Schema { user_account, .. } = schema;

    // Insert test data with different enum values
    let test_users = vec![
        InsertUserAccount::new("guest_user", UserRole::Guest, AccountStatus::Inactive),
        InsertUserAccount::new("member_user", UserRole::Member, AccountStatus::Active),
        InsertUserAccount::new("admin_user", UserRole::Admin, AccountStatus::Suspended),
    ];

    let inserted = db.insert(user_account).values(test_users).execute();
    assert_eq!(inserted, 3);

    // Select and verify the data
    let results: Vec<UserAccountResult> = db
        .select((
            user_account.id,
            user_account.name,
            user_account.role,
            user_account.status,
        ))
        .from(user_account)
        .all();

    assert_eq!(results.len(), 3);

    // Verify guest user - enum types work for all drivers
    let guest = results.iter().find(|u| u.name == "guest_user").unwrap();
    assert_eq!(guest.role, UserRole::Guest);
    assert_eq!(guest.status, AccountStatus::Inactive);

    // Verify member user
    let member = results.iter().find(|u| u.name == "member_user").unwrap();
    assert_eq!(member.role, UserRole::Member);
    assert_eq!(member.status, AccountStatus::Active);

    // Verify admin user
    let admin = results.iter().find(|u| u.name == "admin_user").unwrap();
    assert_eq!(admin.role, UserRole::Admin);
    assert_eq!(admin.status, AccountStatus::Suspended);

    // Test filtering by enum values
    let admin_users: Vec<UserAccountResult> = db
        .select((
            user_account.id,
            user_account.name,
            user_account.role,
            user_account.status,
        ))
        .from(user_account)
        .r#where(eq(UserAccount::role, UserRole::Admin))
        .all();

    assert_eq!(admin_users.len(), 1);
    assert_eq!(admin_users[0].name, "admin_user");

    // Test filtering by integer enum
    let suspended_users: Vec<UserAccountResult> = db
        .select((
            user_account.id,
            user_account.name,
            user_account.role,
            user_account.status,
        ))
        .from(user_account)
        .r#where(eq(UserAccount::status, AccountStatus::Suspended))
        .all();

    assert_eq!(suspended_users.len(), 1);
    assert_eq!(suspended_users[0].name, "admin_user");
}

#[drizzle::test]
fn repr_enum_shared_by_two_tables_filters_and_round_trips(db: &mut TestDb<Schema>) {
    let Schema {
        primary_job,
        secondary_job,
        ..
    } = schema;

    let primary_inserted = db
        .insert(primary_job)
        .values([
            InsertPrimaryJob::new(SharedJobStatus::Queued),
            InsertPrimaryJob::new(SharedJobStatus::Complete),
        ])
        .execute();
    let secondary_inserted = db
        .insert(secondary_job)
        .values([
            InsertSecondaryJob::new(SharedJobStatus::Complete),
            InsertSecondaryJob::new(SharedJobStatus::Queued),
        ])
        .execute();
    assert_eq!(primary_inserted, 2);
    assert_eq!(secondary_inserted, 2);

    let primary: Vec<SelectPrimaryJob> = db
        .select(())
        .from(primary_job)
        .r#where(eq(primary_job.status, SharedJobStatus::Complete))
        .all();
    let secondary: Vec<SelectSecondaryJob> = db
        .select(())
        .from(secondary_job)
        .r#where(eq(secondary_job.status, SharedJobStatus::Queued))
        .all();

    assert_eq!(primary.len(), 1);
    assert_eq!(primary[0].status, SharedJobStatus::Complete);
    assert_eq!(secondary.len(), 1);
    assert_eq!(secondary[0].status, SharedJobStatus::Queued);
}

#[cfg(feature = "query")]
#[drizzle::test]
fn marker_free_enum_query_uses_enum_owned_decoder(db: &mut TestDb<Schema>) {
    let Schema { primary_job, .. } = schema;
    db.insert(primary_job)
        .values([InsertPrimaryJob::new(SharedJobStatus::Complete)])
        .execute();

    let rows = db.query(primary_job).find_many();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, SharedJobStatus::Complete);
}
