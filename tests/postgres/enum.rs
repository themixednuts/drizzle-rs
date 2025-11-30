//! PostgreSQL enum tests

#![cfg(feature = "postgres")]

use crate::common::pg::*;
use drizzle::prelude::*;

#[test]
fn test_sql_enum_info_name() {
    let role = PgRole::new();
    assert_eq!(role.name(), "PgRole");
}

#[test]
fn test_sql_enum_info_variants() {
    let role = PgRole::new();
    let variants = role.variants();

    assert_eq!(variants.len(), 3);
    assert!(variants.contains(&"User"));
    assert!(variants.contains(&"Admin"));
    assert!(variants.contains(&"Moderator"));
}

#[test]
fn test_sql_enum_info_create_type_sql() {
    let role = PgRole::new();
    let sql = role.create_type_sql();

    println!("CREATE TYPE SQL: {}", sql);

    assert!(sql.contains("CREATE TYPE"));
    assert!(sql.contains("PgRole"));
    assert!(sql.contains("AS ENUM"));
    assert!(sql.contains("'User'"));
    assert!(sql.contains("'Admin'"));
    assert!(sql.contains("'Moderator'"));
}

#[test]
fn test_enum_from_str() {
    let user: PgRole = "User".parse().expect("Should parse User");
    let admin: PgRole = "Admin".parse().expect("Should parse Admin");
    let moderator: PgRole = "Moderator".parse().expect("Should parse Moderator");

    assert!(matches!(user, PgRole::User));
    assert!(matches!(admin, PgRole::Admin));
    assert!(matches!(moderator, PgRole::Moderator));
}

#[test]
fn test_enum_from_str_error() {
    let result: Result<PgRole, _> = "Invalid".parse();
    assert!(result.is_err());
}

#[test]
fn test_enum_try_from_str() {
    let user = PgRole::try_from("User").expect("Should convert User");
    assert!(matches!(user, PgRole::User));

    let invalid = PgRole::try_from("Invalid");
    assert!(invalid.is_err());
}

#[test]
fn test_enum_try_from_string() {
    let user = PgRole::try_from("User".to_string()).expect("Should convert User");
    assert!(matches!(user, PgRole::User));
}

#[test]
fn test_enum_display() {
    assert_eq!(PgRole::User.to_string(), "User");
    assert_eq!(PgRole::Admin.to_string(), "Admin");
    assert_eq!(PgRole::Moderator.to_string(), "Moderator");
}

#[test]
fn test_enum_as_ref_str() {
    let user: &str = PgRole::User.as_ref();
    assert_eq!(user, "User");
}

#[test]
fn test_enum_into_str() {
    let user: &str = PgRole::User.into();
    assert_eq!(user, "User");
}

#[test]
fn test_enum_clone() {
    let original = PgRole::Admin;
    let cloned = original.clone();
    assert!(matches!(cloned, PgRole::Admin));
}

#[test]
fn test_enum_default() {
    let default = PgRole::default();
    assert!(matches!(default, PgRole::User));
}

#[test]
fn test_enum_from_i64() {
    let user = PgRole::try_from(0i64).expect("Should convert 0 to User");
    let admin = PgRole::try_from(1i64).expect("Should convert 1 to Admin");
    let moderator = PgRole::try_from(2i64).expect("Should convert 2 to Moderator");

    assert!(matches!(user, PgRole::User));
    assert!(matches!(admin, PgRole::Admin));
    assert!(matches!(moderator, PgRole::Moderator));
}

#[test]
fn test_enum_from_i64_error() {
    let result = PgRole::try_from(99i64);
    assert!(result.is_err());
}

#[test]
fn test_enum_to_i64() {
    let user: i64 = PgRole::User.into();
    let admin: i64 = PgRole::Admin.into();
    let moderator: i64 = PgRole::Moderator.into();

    assert_eq!(user, 0);
    assert_eq!(admin, 1);
    assert_eq!(moderator, 2);
}

#[test]
fn test_enum_from_integer_types() {
    // Test from various integer types
    let from_i32: PgRole = PgRole::try_from(1i32 as i64).expect("Should convert i32");
    let from_i16: PgRole = PgRole::try_from(1i16 as i64).expect("Should convert i16");

    assert!(matches!(from_i32, PgRole::Admin));
    assert!(matches!(from_i16, PgRole::Admin));
}

#[test]
fn test_priority_enum_sql() {
    let priority = Priority::new();
    let sql = priority.create_type_sql();

    println!("Priority CREATE TYPE SQL: {}", sql);

    assert!(sql.contains("CREATE TYPE"));
    assert!(sql.contains("Priority"));
    assert!(sql.contains("'Low'"));
    assert!(sql.contains("'Medium'"));
    assert!(sql.contains("'High'"));
}

#[test]
fn test_post_status_enum_sql() {
    let status = PostStatus::new();
    let sql = status.create_type_sql();

    println!("PostStatus CREATE TYPE SQL: {}", sql);

    assert!(sql.contains("CREATE TYPE"));
    assert!(sql.contains("PostStatus"));
    assert!(sql.contains("'Draft'"));
    assert!(sql.contains("'Published'"));
    assert!(sql.contains("'Archived'"));
}

#[test]
fn test_enum_with_explicit_discriminants() {
    // PgRole uses default discriminants (0, 1, 2)
    assert_eq!(i64::from(PgRole::User), 0);
    assert_eq!(i64::from(PgRole::Admin), 1);
    assert_eq!(i64::from(PgRole::Moderator), 2);
}

#[test]
fn test_enum_from_explicit_discriminants() {
    let user = PgRole::try_from(0i64).expect("Should convert 0");
    let admin = PgRole::try_from(1i64).expect("Should convert 1");

    assert!(matches!(user, PgRole::User));
    assert!(matches!(admin, PgRole::Admin));
}
