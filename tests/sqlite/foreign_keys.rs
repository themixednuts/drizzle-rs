#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
use crate::common::schema::sqlite::Role;
use drizzle::core::conditions::*;
use drizzle::sqlite::prelude::*;
use drizzle_macros::sqlite_test;

#[cfg(feature = "uuid")]
use uuid::Uuid;

#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::{Complex, InsertComplex, InsertPost, Post, SelectPost};
#[cfg(not(feature = "uuid"))]
use crate::common::schema::sqlite::{FullBlogSchema, InsertPost, Post, SelectPost};

//------------------------------------------------------------------------------
// Foreign Key Action Type Schema Definitions
// Note: SQLite doesn't have SERIAL, so we use autoincrement for auto-generated IDs
//------------------------------------------------------------------------------

/// Parent table for foreign key action tests
#[SQLiteTable]
pub struct FkParent {
    #[column(primary, autoincrement)]
    pub id: i32,
    pub name: String,
}

/// Test ON DELETE CASCADE action
#[SQLiteTable]
pub struct FkCascade {
    #[column(primary, autoincrement)]
    pub id: i32,
    #[column(references = FkParent::id, on_delete = CASCADE)]
    pub parent_id: Option<i32>,
    pub value: String,
}

/// Test ON DELETE SET NULL action
#[SQLiteTable]
pub struct FkSetNull {
    #[column(primary, autoincrement)]
    pub id: i32,
    #[column(references = FkParent::id, on_delete = SET_NULL)]
    pub parent_id: Option<i32>,
    pub value: String,
}

/// Test ON DELETE SET DEFAULT action
#[SQLiteTable]
pub struct FkSetDefault {
    #[column(primary, autoincrement)]
    pub id: i32,
    #[column(references = FkParent::id, on_delete = SET_DEFAULT, default = 0)]
    pub parent_id: i32,
    pub value: String,
}

/// Test ON DELETE RESTRICT action
#[SQLiteTable]
pub struct FkRestrict {
    #[column(primary, autoincrement)]
    pub id: i32,
    #[column(references = FkParent::id, on_delete = RESTRICT)]
    pub parent_id: Option<i32>,
    pub value: String,
}

/// Test ON DELETE NO ACTION action
#[SQLiteTable]
pub struct FkNoAction {
    #[column(primary, autoincrement)]
    pub id: i32,
    #[column(references = FkParent::id, on_delete = NO_ACTION)]
    pub parent_id: Option<i32>,
    pub value: String,
}

/// Test ON UPDATE CASCADE action
#[SQLiteTable]
pub struct FkUpdateCascade {
    #[column(primary, autoincrement)]
    pub id: i32,
    #[column(references = FkParent::id, on_update = CASCADE)]
    pub parent_id: Option<i32>,
    pub value: String,
}

/// Test ON UPDATE SET NULL action
#[SQLiteTable]
pub struct FkUpdateSetNull {
    #[column(primary, autoincrement)]
    pub id: i32,
    #[column(references = FkParent::id, on_update = SET_NULL)]
    pub parent_id: Option<i32>,
    pub value: String,
}

/// Test both ON DELETE and ON UPDATE together
#[SQLiteTable]
pub struct FkBothActions {
    #[column(primary, autoincrement)]
    pub id: i32,
    #[column(references = FkParent::id, on_delete = CASCADE, on_update = SET_NULL)]
    pub parent_id: Option<i32>,
    pub value: String,
}

//------------------------------------------------------------------------------
// Schema Definitions for Tests
//------------------------------------------------------------------------------

#[derive(SQLiteSchema)]
pub struct FkCascadeSchema {
    pub fk_parent: FkParent,
    pub fk_cascade: FkCascade,
}

#[derive(SQLiteSchema)]
pub struct FkSetNullSchema {
    pub fk_parent: FkParent,
    pub fk_set_null: FkSetNull,
}

#[derive(SQLiteSchema)]
pub struct FkSetDefaultSchema {
    pub fk_parent: FkParent,
    pub fk_set_default: FkSetDefault,
}

#[derive(SQLiteSchema)]
pub struct FkRestrictSchema {
    pub fk_parent: FkParent,
    pub fk_restrict: FkRestrict,
}

#[derive(SQLiteSchema)]
pub struct FkNoActionSchema {
    pub fk_parent: FkParent,
    pub fk_no_action: FkNoAction,
}

#[derive(SQLiteSchema)]
pub struct FkUpdateCascadeSchema {
    pub fk_parent: FkParent,
    pub fk_update_cascade: FkUpdateCascade,
}

#[derive(SQLiteSchema)]
pub struct FkUpdateSetNullSchema {
    pub fk_parent: FkParent,
    pub fk_update_set_null: FkUpdateSetNull,
}

#[derive(SQLiteSchema)]
pub struct FkBothActionsSchema {
    pub fk_parent: FkParent,
    pub fk_both_actions: FkBothActions,
}

//------------------------------------------------------------------------------
// Result Types
//------------------------------------------------------------------------------

#[derive(Debug, SQLiteFromRow)]
struct ParentResult {
    id: i32,
    name: String,
}

#[derive(Debug, SQLiteFromRow)]
struct ChildResult {
    id: i32,
    parent_id: Option<i32>,
    value: String,
}

#[derive(Debug, SQLiteFromRow)]
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
    let table = FkCascade::default();
    let sql = table.sql().sql();
    println!("FkCascade SQL: {}", sql);

    assert!(
        sql.contains("ON DELETE CASCADE"),
        "Should contain ON DELETE CASCADE. Got: {}",
        sql
    );
}

#[test]
fn test_on_delete_set_null_sql() {
    let table = FkSetNull::default();
    let sql = table.sql().sql();
    println!("FkSetNull SQL: {}", sql);

    assert!(
        sql.contains("ON DELETE SET NULL"),
        "Should contain ON DELETE SET NULL. Got: {}",
        sql
    );
}

#[test]
fn test_on_delete_set_default_sql() {
    let table = FkSetDefault::default();
    let sql = table.sql().sql();
    println!("FkSetDefault SQL: {}", sql);

    assert!(
        sql.contains("ON DELETE SET DEFAULT"),
        "Should contain ON DELETE SET DEFAULT. Got: {}",
        sql
    );
}

#[test]
fn test_on_delete_restrict_sql() {
    let table = FkRestrict::default();
    let sql = table.sql().sql();
    println!("FkRestrict SQL: {}", sql);

    assert!(
        sql.contains("ON DELETE RESTRICT"),
        "Should contain ON DELETE RESTRICT. Got: {}",
        sql
    );
}

#[test]
fn test_on_delete_no_action_sql() {
    let table = FkNoAction::default();
    let sql = table.sql().sql();
    println!("FkNoAction SQL: {}", sql);

    assert!(
        sql.contains("ON DELETE NO ACTION"),
        "Should contain ON DELETE NO ACTION. Got: {}",
        sql
    );
}

#[test]
fn test_on_update_cascade_sql() {
    let table = FkUpdateCascade::default();
    let sql = table.sql().sql();
    println!("FkUpdateCascade SQL: {}", sql);

    assert!(
        sql.contains("ON UPDATE CASCADE"),
        "Should contain ON UPDATE CASCADE. Got: {}",
        sql
    );
}

#[test]
fn test_on_update_set_null_sql() {
    let table = FkUpdateSetNull::default();
    let sql = table.sql().sql();
    println!("FkUpdateSetNull SQL: {}", sql);

    assert!(
        sql.contains("ON UPDATE SET NULL"),
        "Should contain ON UPDATE SET NULL. Got: {}",
        sql
    );
}

#[test]
fn test_both_actions_sql() {
    let table = FkBothActions::default();
    let sql = table.sql().sql();
    println!("FkBothActions SQL: {}", sql);

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
fn test_foreign_key_reference_sql() {
    let post_instance = Post::default();
    let post_sql = post_instance.sql().sql();

    println!("Post table SQL: {}", post_sql);

    assert!(post_sql.contains("CREATE TABLE"));
    assert!(post_sql.contains("posts"));

    // Check for foreign key constraint
    assert!(
        post_sql.contains("REFERENCES"),
        "Post table should contain REFERENCES for foreign key"
    );
    assert!(
        post_sql.contains("complex"),
        "Post table should reference complex table"
    );
    assert!(
        post_sql.contains("(id)"),
        "Post table should reference id column"
    );

    // Note: The common Post schema doesn't define ON DELETE/ON UPDATE actions
    // Those are tested separately in the dedicated action tests above
}

//------------------------------------------------------------------------------
// ON DELETE Integration Tests
// Note: With autoincrement, id is auto-generated so new() only needs required non-default fields
//------------------------------------------------------------------------------

sqlite_test!(test_cascade_deletes_children, FkCascadeSchema, {
    let FkCascadeSchema {
        fk_parent,
        fk_cascade,
    } = schema;

    // Insert parent record (id is autoincrement, so only name is required)
    drizzle_exec!(
        db.insert(fk_parent)
            .values([InsertFkParent::new("Parent1")])
            .execute()
    );

    // Get the auto-generated parent ID
    let parents: Vec<ParentResult> = drizzle_exec!(db.select(()).from(fk_parent).all());
    let parent_id = parents[0].id;

    // Insert child (id is autoincrement, value is required, parent_id is optional)
    drizzle_exec!(
        db.insert(fk_cascade)
            .values([InsertFkCascade::new("Child1").with_parent_id(parent_id)])
            .execute()
    );

    // Verify child exists
    let children: Vec<ChildResult> = drizzle_exec!(db.select(()).from(fk_cascade).all());
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].parent_id, Some(parent_id));

    // Delete parent - should cascade delete child
    drizzle_exec!(
        db.delete(fk_parent)
            .r#where(eq(fk_parent.id, parent_id))
            .execute()
    );

    // Verify child was deleted by cascade
    let children: Vec<ChildResult> = drizzle_exec!(db.select(()).from(fk_cascade).all());
    assert_eq!(children.len(), 0, "Child should be deleted by CASCADE");
});

sqlite_test!(test_set_null_nullifies_children, FkSetNullSchema, {
    let FkSetNullSchema {
        fk_parent,
        fk_set_null,
    } = schema;

    // Insert parent
    drizzle_exec!(
        db.insert(fk_parent)
            .values([InsertFkParent::new("Parent1")])
            .execute()
    );

    // Get the auto-generated parent ID
    let parents: Vec<ParentResult> = drizzle_exec!(db.select(()).from(fk_parent).all());
    let parent_id = parents[0].id;

    // Insert child referencing the parent
    drizzle_exec!(
        db.insert(fk_set_null)
            .values([InsertFkSetNull::new("Child1").with_parent_id(parent_id)])
            .execute()
    );

    // Verify child exists with parent_id set
    let children: Vec<ChildResult> = drizzle_exec!(db.select(()).from(fk_set_null).all());
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].parent_id, Some(parent_id));

    // Delete parent - should set child's parent_id to NULL
    drizzle_exec!(
        db.delete(fk_parent)
            .r#where(eq(fk_parent.id, parent_id))
            .execute()
    );

    // Verify child still exists but parent_id is NULL
    let children: Vec<ChildResult> = drizzle_exec!(db.select(()).from(fk_set_null).all());
    assert_eq!(children.len(), 1, "Child should still exist");
    assert_eq!(
        children[0].parent_id, None,
        "Parent ID should be NULL after SET NULL"
    );
});

sqlite_test!(test_set_default_sets_default_value, FkSetDefaultSchema, {
    let FkSetDefaultSchema {
        fk_parent,
        fk_set_default,
    } = schema;

    // Insert default parent - we'll use its id as the default (note: using with_id to set specific id)
    drizzle_exec!(
        db.insert(fk_parent)
            .values([InsertFkParent::new("DefaultParent").with_id(0)])
            .execute()
    );

    // Insert parent that we'll delete
    drizzle_exec!(
        db.insert(fk_parent)
            .values([InsertFkParent::new("Parent1")])
            .execute()
    );

    // Get the non-default parent's ID
    let parents: Vec<ParentResult> = drizzle_exec!(
        db.select(())
            .from(fk_parent)
            .r#where(eq(fk_parent.name, "Parent1"))
            .all()
    );
    let parent_id = parents[0].id;

    // Insert child referencing parent (parent_id has DEFAULT = 0, but we override it)
    drizzle_exec!(
        db.insert(fk_set_default)
            .values([InsertFkSetDefault::new("Child1").with_parent_id(parent_id)])
            .execute()
    );

    // Verify child has parent_id = parent_id
    let children: Vec<ChildDefaultResult> = drizzle_exec!(db.select(()).from(fk_set_default).all());
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].parent_id, parent_id);

    // Delete the parent - should set child's parent_id to default (0)
    drizzle_exec!(
        db.delete(fk_parent)
            .r#where(eq(fk_parent.id, parent_id))
            .execute()
    );

    // Verify child's parent_id is now the default value (0)
    let children: Vec<ChildDefaultResult> = drizzle_exec!(db.select(()).from(fk_set_default).all());
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

sqlite_test!(
    test_update_cascade_updates_children,
    FkUpdateCascadeSchema,
    {
        let FkUpdateCascadeSchema {
            fk_parent,
            fk_update_cascade,
        } = schema;

        // Insert parent
        drizzle_exec!(
            db.insert(fk_parent)
                .values([InsertFkParent::new("Parent1")])
                .execute()
        );

        // Get the auto-generated parent ID
        let parents: Vec<ParentResult> = drizzle_exec!(db.select(()).from(fk_parent).all());
        let parent_id = parents[0].id;

        // Insert child referencing the parent
        drizzle_exec!(
            db.insert(fk_update_cascade)
                .values([InsertFkUpdateCascade::new("Child1").with_parent_id(parent_id)])
                .execute()
        );

        // Verify child has the parent_id
        let children: Vec<ChildResult> = drizzle_exec!(db.select(()).from(fk_update_cascade).all());
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].parent_id, Some(parent_id));

        // Update parent's id to 100 - should cascade update child
        drizzle_exec!(
            db.update(fk_parent)
                .set(UpdateFkParent::default().with_id(100))
                .r#where(eq(fk_parent.id, parent_id))
                .execute()
        );

        // Verify child's parent_id was cascaded to 100
        let children: Vec<ChildResult> = drizzle_exec!(db.select(()).from(fk_update_cascade).all());
        assert_eq!(children.len(), 1);
        assert_eq!(
            children[0].parent_id,
            Some(100),
            "Child's parent_id should be updated by CASCADE"
        );
    }
);

sqlite_test!(
    test_update_set_null_nullifies_children,
    FkUpdateSetNullSchema,
    {
        let FkUpdateSetNullSchema {
            fk_parent,
            fk_update_set_null,
        } = schema;

        // Insert parent
        drizzle_exec!(
            db.insert(fk_parent)
                .values([InsertFkParent::new("Parent1")])
                .execute()
        );

        // Get the auto-generated parent ID
        let parents: Vec<ParentResult> = drizzle_exec!(db.select(()).from(fk_parent).all());
        let parent_id = parents[0].id;

        // Insert child referencing the parent
        drizzle_exec!(
            db.insert(fk_update_set_null)
                .values([InsertFkUpdateSetNull::new("Child1").with_parent_id(parent_id)])
                .execute()
        );

        // Verify child has the parent_id
        let children: Vec<ChildResult> =
            drizzle_exec!(db.select(()).from(fk_update_set_null).all());
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].parent_id, Some(parent_id));

        // Update parent's id to 100 - should set child's parent_id to NULL
        drizzle_exec!(
            db.update(fk_parent)
                .set(UpdateFkParent::default().with_id(100))
                .r#where(eq(fk_parent.id, parent_id))
                .execute()
        );

        // Verify child's parent_id is now NULL
        let children: Vec<ChildResult> =
            drizzle_exec!(db.select(()).from(fk_update_set_null).all());
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

sqlite_test!(
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
                    InsertFkParent::new("Parent1"),
                    InsertFkParent::new("Parent2"),
                ])
                .execute()
        );

        // Get the auto-generated parent IDs
        let parents: Vec<ParentResult> = drizzle_exec!(db.select(()).from(fk_parent).all());
        let parent1_id = parents.iter().find(|p| p.name == "Parent1").unwrap().id;
        let parent2_id = parents.iter().find(|p| p.name == "Parent2").unwrap().id;

        // Insert children referencing each parent
        drizzle_exec!(
            db.insert(fk_both_actions)
                .values([
                    InsertFkBothActions::new("Child1").with_parent_id(parent1_id),
                    InsertFkBothActions::new("Child2").with_parent_id(parent2_id),
                ])
                .execute()
        );

        // Test ON UPDATE SET NULL: Update parent1's id using UpdateModel
        drizzle_exec!(
            db.update(fk_parent)
                .set(UpdateFkParent::default().with_id(100))
                .r#where(eq(fk_parent.id, parent1_id))
                .execute()
        );

        // Verify child1's parent_id is NULL (ON UPDATE SET NULL)
        let children: Vec<ChildResult> = drizzle_exec!(
            db.select(())
                .from(fk_both_actions)
                .r#where(eq(fk_both_actions.value, "Child1"))
                .all()
        );
        assert_eq!(
            children[0].parent_id, None,
            "ON UPDATE SET NULL should nullify parent_id"
        );

        // Test ON DELETE CASCADE: Delete parent2
        drizzle_exec!(
            db.delete(fk_parent)
                .r#where(eq(fk_parent.id, parent2_id))
                .execute()
        );

        // Verify child2 was deleted (ON DELETE CASCADE)
        let children: Vec<ChildResult> = drizzle_exec!(
            db.select(())
                .from(fk_both_actions)
                .r#where(eq(fk_both_actions.value, "Child2"))
                .all()
        );
        assert_eq!(children.len(), 0, "ON DELETE CASCADE should delete child2");

        // Child1 should still exist (parent was updated, not deleted)
        let children: Vec<ChildResult> = drizzle_exec!(db.select(()).from(fk_both_actions).all());
        assert_eq!(children.len(), 1, "Child1 should still exist");
    }
);

//------------------------------------------------------------------------------
// Original Complex/Post Schema Integration Tests
//------------------------------------------------------------------------------

#[cfg(feature = "uuid")]
#[derive(SQLiteSchema)]
pub struct ComplexPostSchema {
    pub complex: Complex,
    pub post: Post,
}

#[cfg(feature = "uuid")]
sqlite_test!(test_foreign_key_impl, ComplexPostSchema, {
    let ComplexPostSchema { complex, post } = schema;

    let id = Uuid::new_v4();

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("John", false, Role::User).with_id(id)])
            .execute()
    );

    drizzle_exec!(
        db.insert(post)
            .values([InsertPost::new("test", true).with_author_id(id)])
            .execute()
    );

    let row: SelectPost = drizzle_exec!(
        db.select(())
            .from(post)
            .r#where(eq(post.author_id, id))
            .get()
    );

    assert_eq!(row.author_id, Some(id));
});
