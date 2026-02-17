//! PostgreSQL foreign key tests
//!
//! Tests for ON_DELETE and ON_UPDATE referential actions.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use drizzle::core::expr::*;
use drizzle::postgres::prelude::*;
use drizzle_macros::postgres_test;

//------------------------------------------------------------------------------
// Foreign Key Action Type Schema Definitions
//------------------------------------------------------------------------------

/// Parent table for foreign key action tests
#[PostgresTable]
pub struct FkParent {
    #[column(primary)]
    pub id: i32,
    pub name: String,
}

/// Test ON DELETE CASCADE action
#[PostgresTable]
pub struct FkCascade {
    #[column(serial, primary)]
    pub id: i32,
    #[column(REFERENCES = FkParent::id, ON_DELETE = CASCADE)]
    pub parent_id: Option<i32>,
    pub value: String,
}

/// Test ON DELETE SET NULL action
#[PostgresTable]
pub struct FkSetNull {
    #[column(serial, primary)]
    pub id: i32,
    #[column(REFERENCES = FkParent::id, ON_DELETE = SET_NULL)]
    pub parent_id: Option<i32>,
    pub value: String,
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

/// Test ON DELETE RESTRICT action  
#[PostgresTable]
pub struct FkRestrict {
    #[column(serial, primary)]
    pub id: i32,
    #[column(REFERENCES = FkParent::id, ON_DELETE = RESTRICT)]
    pub parent_id: Option<i32>,
    pub value: String,
}

/// Test ON DELETE NO ACTION action
#[PostgresTable]
pub struct FkNoAction {
    #[column(serial, primary)]
    pub id: i32,
    #[column(REFERENCES = FkParent::id, ON_DELETE = NO_ACTION)]
    pub parent_id: Option<i32>,
    pub value: String,
}

/// Test ON UPDATE CASCADE action
#[PostgresTable]
pub struct FkUpdateCascade {
    #[column(serial, primary)]
    pub id: i32,
    #[column(REFERENCES = FkParent::id, ON_UPDATE = CASCADE)]
    pub parent_id: Option<i32>,
    pub value: String,
}

/// Test ON UPDATE SET NULL action
#[PostgresTable]
pub struct FkUpdateSetNull {
    #[column(serial, primary)]
    pub id: i32,
    #[column(REFERENCES = FkParent::id, ON_UPDATE = SET_NULL)]
    pub parent_id: Option<i32>,
    pub value: String,
}

/// Test both ON DELETE and ON UPDATE together
#[PostgresTable]
pub struct FkBothActions {
    #[column(serial, primary)]
    pub id: i32,
    #[column(REFERENCES = FkParent::id, ON_DELETE = CASCADE, ON_UPDATE = SET_NULL)]
    pub parent_id: Option<i32>,
    pub value: String,
}

#[PostgresTable]
pub struct CompositeFkParent {
    #[column(primary)]
    pub id_a: i32,
    #[column(primary)]
    pub id_b: i32,
    pub label: String,
}

#[PostgresTable(FOREIGN_KEY(
    columns(parent_a, parent_b),
    references(CompositeFkParent, id_a, id_b),
    on_delete = "CASCADE",
    on_update = "CASCADE"
))]
pub struct CompositeFkChild {
    #[column(serial, primary)]
    pub id: i32,
    pub parent_a: Option<i32>,
    pub parent_b: Option<i32>,
    pub value: String,
}

//------------------------------------------------------------------------------
// Schema Definitions for Tests
//------------------------------------------------------------------------------

#[derive(PostgresSchema)]
pub struct FkCascadeSchema {
    pub fk_parent: FkParent,
    pub fk_cascade: FkCascade,
}

#[derive(PostgresSchema)]
pub struct FkSetNullSchema {
    pub fk_parent: FkParent,
    pub fk_set_null: FkSetNull,
}

#[derive(PostgresSchema)]
pub struct FkSetDefaultSchema {
    pub fk_parent: FkParent,
    pub fk_set_default: FkSetDefault,
}

#[derive(PostgresSchema)]
pub struct FkRestrictSchema {
    pub fk_parent: FkParent,
    pub fk_restrict: FkRestrict,
}

#[derive(PostgresSchema)]
pub struct FkNoActionSchema {
    pub fk_parent: FkParent,
    pub fk_no_action: FkNoAction,
}

#[derive(PostgresSchema)]
pub struct FkUpdateCascadeSchema {
    pub fk_parent: FkParent,
    pub fk_update_cascade: FkUpdateCascade,
}

#[derive(PostgresSchema)]
pub struct FkUpdateSetNullSchema {
    pub fk_parent: FkParent,
    pub fk_update_set_null: FkUpdateSetNull,
}

#[derive(PostgresSchema)]
pub struct FkBothActionsSchema {
    pub fk_parent: FkParent,
    pub fk_both_actions: FkBothActions,
}

#[derive(PostgresSchema)]
pub struct CompositeFkSchema {
    pub composite_fk_parent: CompositeFkParent,
    pub composite_fk_child: CompositeFkChild,
}

//------------------------------------------------------------------------------
// Result Types
//------------------------------------------------------------------------------

#[allow(dead_code)]
#[derive(Debug, PostgresFromRow)]
struct ChildResult {
    id: i32,
    parent_id: Option<i32>,
    value: String,
}

#[allow(dead_code)]
#[derive(Debug, PostgresFromRow)]
struct ChildDefaultResult {
    id: i32,
    parent_id: i32,
    value: String,
}

//------------------------------------------------------------------------------
// SQL Generation Tests
//------------------------------------------------------------------------------

#[test]
fn test_on_delete_cascade_sql() {
    let sql = FkCascade::create_table_sql();

    assert!(
        sql.contains("ON DELETE CASCADE"),
        "Should contain ON DELETE CASCADE. Got: {}",
        sql
    );
}

#[test]
fn test_on_delete_set_null_sql() {
    let sql = FkSetNull::create_table_sql();

    assert!(
        sql.contains("ON DELETE SET NULL"),
        "Should contain ON DELETE SET NULL. Got: {}",
        sql
    );
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

#[test]
fn test_on_delete_restrict_sql() {
    let sql = FkRestrict::create_table_sql();

    assert!(
        sql.contains("ON DELETE RESTRICT"),
        "Should contain ON DELETE RESTRICT. Got: {}",
        sql
    );
}

#[test]
fn test_on_delete_no_action_sql() {
    let sql = FkNoAction::create_table_sql();

    // NO ACTION is the default, so it may not appear explicitly in the SQL
    // Just verify the FK constraint references the parent table
    assert!(
        sql.contains("FOREIGN KEY") && sql.contains("REFERENCES"),
        "Should contain FOREIGN KEY REFERENCES. Got: {}",
        sql
    );
}

#[test]
fn test_on_update_cascade_sql() {
    let sql = FkUpdateCascade::create_table_sql();

    assert!(
        sql.contains("ON UPDATE CASCADE"),
        "Should contain ON UPDATE CASCADE. Got: {}",
        sql
    );
}

#[test]
fn test_on_update_set_null_sql() {
    let sql = FkUpdateSetNull::create_table_sql();

    assert!(
        sql.contains("ON UPDATE SET NULL"),
        "Should contain ON UPDATE SET NULL. Got: {}",
        sql
    );
}

#[test]
fn test_both_actions_sql() {
    let sql = FkBothActions::create_table_sql();

    assert!(
        sql.contains("ON DELETE CASCADE"),
        "Should contain ON DELETE CASCADE. Got: {}",
        sql
    );
    assert!(
        sql.contains("ON UPDATE SET NULL"),
        "Should contain ON UPDATE SET NULL. Got: {}",
        sql
    );
}

#[test]
fn test_composite_foreign_key_sql() {
    let sql = CompositeFkChild::create_table_sql();

    assert!(
        sql.contains("FOREIGN KEY"),
        "missing FOREIGN KEY clause: {sql}"
    );
    assert!(
        sql.contains("parent_a"),
        "missing parent_a source column: {sql}"
    );
    assert!(
        sql.contains("parent_b"),
        "missing parent_b source column: {sql}"
    );
    assert!(
        sql.contains("REFERENCES"),
        "missing REFERENCES clause: {sql}"
    );
    assert!(sql.contains("id_a"), "missing id_a target column: {sql}");
    assert!(sql.contains("id_b"), "missing id_b target column: {sql}");
    assert!(
        sql.contains("ON DELETE CASCADE"),
        "missing ON DELETE CASCADE action: {sql}"
    );
    assert!(
        sql.contains("ON UPDATE CASCADE"),
        "missing ON UPDATE CASCADE action: {sql}"
    );
}

#[test]
fn test_composite_foreign_key_metadata_grouping() {
    let table = CompositeFkChild::new();
    let fks = table.foreign_keys();

    assert_eq!(fks.len(), 1, "expected a single grouped FK");
    assert_eq!(fks[0].source_columns(), &["parent_a", "parent_b"]);
    assert_eq!(fks[0].target_columns(), &["id_a", "id_b"]);
}

#[test]
fn test_table_constraints_metadata() {
    let parent = CompositeFkParent::new();
    let child = CompositeFkChild::new();

    let parent_constraints = parent.constraints();
    assert!(
        parent_constraints
            .iter()
            .any(|c| c.kind() == SQLConstraintKind::PrimaryKey),
        "expected parent to expose a primary key constraint"
    );

    let child_constraints = child.constraints();
    assert!(
        child_constraints
            .iter()
            .any(|c| c.kind() == SQLConstraintKind::ForeignKey),
        "expected child to expose a foreign key constraint"
    );
}

#[test]
fn test_composite_primary_key_metadata() {
    let table = CompositeFkParent::new();
    let pk = table
        .primary_key()
        .expect("expected composite primary key metadata");

    assert_eq!(pk.columns(), &["id_a", "id_b"]);
}

#[test]
fn test_constraint_capability_markers() {
    fn assert_has_pk<T: HasPrimaryKey>() {}
    fn assert_has_fk<T: HasConstraint<ForeignKeyK>>() {}
    fn assert_has_pk_constraint<T: HasConstraint<PrimaryKeyK>>() {}
    fn assert_joinable<A: Joinable<B>, B>() {}

    assert_has_pk::<CompositeFkParent>();
    assert_has_pk_constraint::<CompositeFkParent>();
    assert_has_fk::<FkCascade>();
    assert_joinable::<FkCascade, FkParent>();
}

//------------------------------------------------------------------------------
// ON DELETE Integration Tests
// Note: FkParent has id (primary, no serial) so new() requires (id, name)
// Child tables have serial id, so new() only requires non-default fields
//------------------------------------------------------------------------------

postgres_test!(test_cascade_deletes_children, FkCascadeSchema, {
    let FkCascadeSchema {
        fk_parent,
        fk_cascade,
    } = schema;

    // Insert parent record (id is required since no serial)
    drizzle_exec!(
        db.insert(fk_parent)
            .values([InsertFkParent::new(1, "Parent1")])
            => execute
    );

    // Insert child record (id is serial, parent_id is optional, value is required)
    drizzle_exec!(
        db.insert(fk_cascade)
            .values([InsertFkCascade::new("Child1").with_parent_id(1)])
            => execute
    );

    // Verify child exists
    let children: Vec<ChildResult> = drizzle_exec!(db.select(()).from(fk_cascade) => all_as);
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].parent_id, Some(1));

    // Delete parent - should cascade delete child
    drizzle_exec!(db.delete(fk_parent).r#where(eq(fk_parent.id, 1)) => execute);

    // Verify child was deleted by cascade
    let children: Vec<ChildResult> = drizzle_exec!(db.select(()).from(fk_cascade) => all_as);
    assert_eq!(children.len(), 0, "Child should be deleted by CASCADE");
});

postgres_test!(test_set_null_nullifies_children, FkSetNullSchema, {
    let FkSetNullSchema {
        fk_parent,
        fk_set_null,
    } = schema;

    // Insert parent
    drizzle_exec!(
        db.insert(fk_parent)
            .values([InsertFkParent::new(1, "Parent1")])
            => execute
    );

    // Insert child referencing the parent
    drizzle_exec!(
        db.insert(fk_set_null)
            .values([InsertFkSetNull::new("Child1").with_parent_id(1)])
            => execute
    );

    // Verify child exists with parent_id set
    let children: Vec<ChildResult> = drizzle_exec!(db.select(()).from(fk_set_null) => all_as);
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].parent_id, Some(1));

    // Delete parent - should set child's parent_id to NULL
    drizzle_exec!(db.delete(fk_parent).r#where(eq(fk_parent.id, 1)) => execute);

    // Verify child still exists but parent_id is NULL
    let children: Vec<ChildResult> = drizzle_exec!(db.select(()).from(fk_set_null) => all_as);
    assert_eq!(children.len(), 1, "Child should still exist");
    assert_eq!(
        children[0].parent_id, None,
        "Parent ID should be NULL after SET NULL"
    );
});

postgres_test!(test_set_default_sets_default_value, FkSetDefaultSchema, {
    let FkSetDefaultSchema {
        fk_parent,
        fk_set_default,
    } = schema;

    // Insert default parent with id=0 (the default value for fk)
    drizzle_exec!(
        db.insert(fk_parent)
            .values([InsertFkParent::new(0, "DefaultParent")])
            => execute
    );

    // Insert parent with id=1
    drizzle_exec!(
        db.insert(fk_parent)
            .values([InsertFkParent::new(1, "Parent1")])
            => execute
    );

    // Insert child referencing parent id=1 (parent_id has default=0, but we set it to 1)
    drizzle_exec!(
        db.insert(fk_set_default)
            .values([InsertFkSetDefault::new("Child1").with_parent_id(1)])
            => execute
    );

    // Verify child has parent_id = 1
    let children: Vec<ChildDefaultResult> =
        drizzle_exec!(db.select(()).from(fk_set_default) => all_as);
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].parent_id, 1);

    // Delete parent with id=1 - should set child's parent_id to default (0)
    drizzle_exec!(db.delete(fk_parent).r#where(eq(fk_parent.id, 1)) => execute);

    // Verify child's parent_id is now the default value (0)
    let children: Vec<ChildDefaultResult> =
        drizzle_exec!(db.select(()).from(fk_set_default) => all_as);
    assert_eq!(children.len(), 1, "Child should still exist");
    assert_eq!(
        children[0].parent_id, 0,
        "Parent ID should be default (0) after SET DEFAULT"
    );
});

//------------------------------------------------------------------------------
// ON UPDATE Integration Tests
// Uses UpdateModel::default().with_field() pattern
//------------------------------------------------------------------------------

postgres_test!(
    test_update_cascade_updates_children,
    FkUpdateCascadeSchema,
    {
        let FkUpdateCascadeSchema {
            fk_parent,
            fk_update_cascade,
        } = schema;

        // Insert parent with id=1
        drizzle_exec!(
            db.insert(fk_parent)
                .values([InsertFkParent::new(1, "Parent1")])
                => execute
        );

        // Insert child referencing the parent
        drizzle_exec!(
            db.insert(fk_update_cascade)
                .values([InsertFkUpdateCascade::new("Child1").with_parent_id(1)])
                => execute
        );

        // Verify child has parent_id = 1
        let children: Vec<ChildResult> =
            drizzle_exec!(db.select(()).from(fk_update_cascade) => all_as);
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].parent_id, Some(1));

        // Update parent's id from 1 to 100 - should cascade update child
        drizzle_exec!(
            db.update(fk_parent)
                .set(UpdateFkParent::default().with_id(100))
                .r#where(eq(fk_parent.id, 1))
                => execute
        );

        // Verify child's parent_id was cascaded to 100
        let children: Vec<ChildResult> =
            drizzle_exec!(db.select(()).from(fk_update_cascade) => all_as);
        assert_eq!(children.len(), 1);
        assert_eq!(
            children[0].parent_id,
            Some(100),
            "Child's parent_id should be updated by CASCADE"
        );
    }
);

postgres_test!(
    test_update_set_null_nullifies_children,
    FkUpdateSetNullSchema,
    {
        let FkUpdateSetNullSchema {
            fk_parent,
            fk_update_set_null,
        } = schema;

        // Insert parent with id=1
        drizzle_exec!(
            db.insert(fk_parent)
                .values([InsertFkParent::new(1, "Parent1")])
                => execute
        );

        // Insert child referencing the parent
        drizzle_exec!(
            db.insert(fk_update_set_null)
                .values([InsertFkUpdateSetNull::new("Child1").with_parent_id(1)])
                => execute
        );

        // Verify child has parent_id = 1
        let children: Vec<ChildResult> =
            drizzle_exec!(db.select(()).from(fk_update_set_null) => all_as);
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].parent_id, Some(1));

        // Update parent's id from 1 to 100 - should set child's parent_id to NULL
        drizzle_exec!(
            db.update(fk_parent)
                .set(UpdateFkParent::default().with_id(100))
                .r#where(eq(fk_parent.id, 1))
                => execute
        );

        // Verify child's parent_id is now NULL
        let children: Vec<ChildResult> =
            drizzle_exec!(db.select(()).from(fk_update_set_null) => all_as);
        assert_eq!(children.len(), 1);
        assert_eq!(
            children[0].parent_id, None,
            "Child's parent_id should be NULL after ON UPDATE SET NULL"
        );
    }
);

//------------------------------------------------------------------------------
// Combined ON DELETE + ON UPDATE Tests
//------------------------------------------------------------------------------

postgres_test!(
    test_both_delete_cascade_and_update_set_null,
    FkBothActionsSchema,
    {
        let FkBothActionsSchema {
            fk_parent,
            fk_both_actions,
        } = schema;

        // Insert parent records
        drizzle_exec!(
            db.insert(fk_parent)
                .values([
                    InsertFkParent::new(1, "Parent1"),
                    InsertFkParent::new(2, "Parent2"),
                ])
                => execute
        );

        // Insert children referencing each parent
        drizzle_exec!(
            db.insert(fk_both_actions)
                .values([
                    InsertFkBothActions::new("Child1").with_parent_id(1),
                    InsertFkBothActions::new("Child2").with_parent_id(2),
                ])
                => execute
        );

        // Test ON UPDATE SET NULL: Update parent1's id using UpdateModel
        drizzle_exec!(
            db.update(fk_parent)
                .set(UpdateFkParent::default().with_id(100))
                .r#where(eq(fk_parent.id, 1))
                => execute
        );

        // Verify child1's parent_id is NULL (ON UPDATE SET NULL)
        let children: Vec<ChildResult> = drizzle_exec!(
            db.select(())
                .from(fk_both_actions)
                .r#where(eq(fk_both_actions.value, "Child1"))
                => all_as
        );
        assert_eq!(
            children[0].parent_id, None,
            "ON UPDATE SET NULL should nullify parent_id"
        );

        // Test ON DELETE CASCADE: Delete parent2
        drizzle_exec!(db.delete(fk_parent).r#where(eq(fk_parent.id, 2)) => execute);

        // Verify child2 was deleted (ON DELETE CASCADE)
        let children: Vec<ChildResult> = drizzle_exec!(
            db.select(())
                .from(fk_both_actions)
                .r#where(eq(fk_both_actions.value, "Child2"))
                => all_as
        );
        assert_eq!(children.len(), 0, "ON DELETE CASCADE should delete child2");

        // Child1 should still exist (parent was updated, not deleted)
        let children: Vec<ChildResult> =
            drizzle_exec!(db.select(()).from(fk_both_actions) => all_as);
        assert_eq!(children.len(), 1, "Child1 should still exist");
    }
);
