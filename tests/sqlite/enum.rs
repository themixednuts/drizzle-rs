#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use drizzle::prelude::*;
use drizzle_macros::drizzle_test;

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

// Table with enum fields using different column types
#[SQLiteTable]
struct UserAccount {
    #[integer(primary_key, autoincrement)]
    id: i64,
    #[text] // This should store UserRole as TEXT
    name: String,
    #[text(enum)] // This should store UserRole as TEXT
    role: UserRole,
    #[integer(enum)] // This should store AccountStatus as INTEGER
    status: AccountStatus,
}

#[derive(SQLiteSchema)]
pub struct Schema {
    user_account: UserAccount,
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
}

#[test]
fn test_table_generation() {
    // Just test that the table compiles and has the expected structure
    let table = UserAccount::new();

    // Test that we can create insert and update models
    let insert_model = InsertUserAccount::new("test", UserRole::Member, AccountStatus::Suspended);
    let update_model = UpdateUserAccount::default();

    // Test convenience methods work with enums
    let _insert_with_role = insert_model.with_role(UserRole::Member);
    let _update_with_status = update_model.with_status(AccountStatus::Suspended);

    // Basic smoke test - if this compiles, the From implementations were generated correctly
    let columns = table.columns();
    assert!(columns.len() > 0);
}

#[derive(Debug, FromRow)]
struct UserAccountResult {
    id: i64,
    name: String,
    role: String, // TEXT representation
    status: i32,  // INTEGER representation
}

drizzle_test!(test_enum_database_roundtrip, Schema, {
    let Schema { user_account } = schema;

    // Insert test data with different enum values
    let test_users = vec![
        InsertUserAccount::new("guest_user", UserRole::Guest, AccountStatus::Inactive),
        InsertUserAccount::new("member_user", UserRole::Member, AccountStatus::Active),
        InsertUserAccount::new("admin_user", UserRole::Admin, AccountStatus::Suspended),
    ];

    let insert_result = db.insert(user_account).values(test_users);
    let sql = insert_result.to_sql();
    println!("{sql}");
    assert_eq!(drizzle_exec!(insert_result.execute()), 3);

    // Select and verify the data
    let results: Vec<UserAccountResult> = drizzle_exec!(
        db.select((
            user_account.id,
            user_account.name,
            user_account.role,
            user_account.status,
        ))
        .from(user_account)
        .all()
    );

    assert_eq!(results.len(), 3);

    // Verify guest user (role as TEXT, status as INTEGER)
    let guest = results.iter().find(|u| u.name == "guest_user").unwrap();
    assert_eq!(guest.role, "Guest"); // TEXT representation
    assert_eq!(guest.status, 3); // INTEGER representation (Inactive = 3)

    // Verify member user
    let member = results.iter().find(|u| u.name == "member_user").unwrap();
    assert_eq!(member.role, "Member");
    assert_eq!(member.status, 4); // Active = 4 (Inactive + 1)

    // Verify admin user
    let admin = results.iter().find(|u| u.name == "admin_user").unwrap();
    assert_eq!(admin.role, "Admin");
    assert_eq!(admin.status, -1); // Suspended = -1

    // Test filtering by enum values
    let admin_users: Vec<UserAccountResult> = drizzle_exec!(
        db.select((
            user_account.id,
            user_account.name,
            user_account.role,
            user_account.status
        ))
        .from(user_account)
        .r#where(eq(UserAccount::role, UserRole::Admin))
        .all()
    );

    assert_eq!(admin_users.len(), 1);
    assert_eq!(admin_users[0].name, "admin_user");

    // Test filtering by integer enum
    let suspended_users: Vec<UserAccountResult> = drizzle_exec!(
        db.select((
            user_account.id,
            user_account.name,
            user_account.role,
            user_account.status
        ))
        .from(user_account)
        .r#where(eq(UserAccount::status, AccountStatus::Suspended))
        .all()
    );

    assert_eq!(suspended_users.len(), 1);
    assert_eq!(suspended_users[0].name, "admin_user");
});
