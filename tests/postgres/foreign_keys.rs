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

/// Referential options may come before `references`; they used to be
/// rejected ("on_delete requires a references attribute") in that order.
#[PostgresTable]
pub struct FkActionsFirst {
    #[column(serial, primary)]
    pub id: i32,
    #[column(ON_DELETE = CASCADE, DEFERRABLE, REFERENCES = FkParent::id)]
    pub parent_id: i32,
}

#[test]
fn referential_options_may_precede_references() {
    let sql = FkActionsFirst::create_table_sql();
    assert!(sql.contains("ON DELETE CASCADE"), "{sql}");
    assert!(sql.contains("DEFERRABLE"), "{sql}");
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

#[PostgresTable]
pub struct FkTenant {
    #[column(primary)]
    pub id: i32,
}

#[PostgresTable]
pub struct FkTenantUser {
    #[column(primary)]
    pub tenant_id: i32,
    #[column(primary)]
    pub id: i32,
}

/// One key on `tenant_id` and one on `(tenant_id, user_id)`. Both used to
/// derive the name `fk_tenant_document_tenant_id_fkey`, so PostgreSQL
/// rejected the table.
#[PostgresTable(FOREIGN_KEY(columns(tenant_id, user_id), references(FkTenantUser, tenant_id, id)))]
pub struct FkTenantDocument {
    #[column(primary)]
    pub id: i32,
    #[column(references = FkTenant::id)]
    pub tenant_id: i32,
    pub user_id: i32,
}

#[derive(PostgresSchema)]
pub struct FkTenantSchema {
    pub tenant: FkTenant,
    pub tenant_user: FkTenantUser,
    pub tenant_document: FkTenantDocument,
}

#[test]
fn foreign_keys_sharing_a_first_column_get_distinct_names() {
    let sql = FkTenantDocument::create_table_sql();
    assert!(
        sql.contains(
            "CONSTRAINT \"fk_tenant_document_tenant_id_fkey\" FOREIGN KEY (\"tenant_id\")"
        ),
        "{sql}"
    );
    assert!(
        sql.contains(
            "CONSTRAINT \"fk_tenant_document_tenant_id_user_id_fkey\" FOREIGN KEY (\"tenant_id\", \"user_id\")"
        ),
        "{sql}"
    );
}

#[drizzle::test]
fn foreign_keys_sharing_a_first_column_are_both_enforced(db: &mut TestDb<FkTenantSchema>) {
    let FkTenantSchema {
        tenant,
        tenant_user,
        tenant_document,
    } = schema;

    db.insert(tenant).values([InsertFkTenant::new(1)]).execute();
    db.insert(tenant_user)
        .values([InsertFkTenantUser::new(1, 10)])
        .execute();
    db.insert(tenant_document)
        .values([InsertFkTenantDocument::new(1, 1, 10)])
        .execute();

    let missing_user = result!(
        db.insert(tenant_document)
            .values([InsertFkTenantDocument::new(2, 1, 11)])
            .execute()
    );
    assert!(missing_user.is_err(), "the composite key must be enforced");
}
