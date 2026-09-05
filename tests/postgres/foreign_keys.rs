//! PostgreSQL foreign keys.
//!
//! The referential-action, composite-key and metadata contracts live in
//! `crate::common::foreign_keys`; this file keeps what MySQL cannot share:
//! `ON DELETE SET DEFAULT`.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use drizzle::core::expr::*;
use drizzle::postgres::prelude::*;

/// Parent table for foreign key action tests
#[PostgresTable]
pub struct FkParent {
    #[column(primary)]
    pub id: i32,
    pub name: String,
}

/// Test ON DELETE SET DEFAULT action
#[PostgresTable]
pub struct FkSetDefault {
    #[column(serial, primary)]
    pub id: i32,
    #[column(REFERENCES = FkParent::id, ON_DELETE = SET_DEFAULT, DEFAULT = 0)]
    pub parent_id: i32,
    pub value: String,
}

#[derive(PostgresSchema)]
pub struct FkSetDefaultSchema {
    pub fk_parent: FkParent,
    pub fk_set_default: FkSetDefault,
}

#[test]
fn test_on_delete_set_default_sql() {
    let sql = FkSetDefault::create_table_sql();

    assert!(
        sql.contains("ON DELETE SET DEFAULT"),
        "Should contain ON DELETE SET DEFAULT. Got: {}",
        sql
    );
}

#[drizzle::test]
fn test_set_default_sets_default_value(db: &mut TestDb<FkSetDefaultSchema>) {
    let FkSetDefaultSchema {
        fk_parent,
        fk_set_default,
    } = schema;

    // Insert default parent with id=0 (the default value for fk)

    db.insert(fk_parent)
        .values([InsertFkParent::new(0, "DefaultParent")])
        .execute();

    // Insert parent with id=1

    db.insert(fk_parent)
        .values([InsertFkParent::new(1, "Parent1")])
        .execute();

    // Insert child referencing parent id=1 (parent_id has default=0, but we set it to 1)

    db.insert(fk_set_default)
        .values([InsertFkSetDefault::new("Child1").with_parent_id(1)])
        .execute();

    // Verify child has parent_id = 1
    let children: Vec<SelectFkSetDefault> = db.select(()).from(fk_set_default).all();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].parent_id, 1);

    // Delete parent with id=1 - should set child's parent_id to default (0)
    db.delete(fk_parent).r#where(eq(fk_parent.id, 1)).execute();

    // Verify child's parent_id is now the default value (0)
    let children: Vec<SelectFkSetDefault> = db.select(()).from(fk_set_default).all();
    assert_eq!(children.len(), 1, "Child should still exist");
    assert_eq!(
        children[0].parent_id, 0,
        "Parent ID should be default (0) after SET DEFAULT"
    );
}
