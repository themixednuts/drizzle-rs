#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

//! SQLite foreign keys.
//!
//! The referential-action, composite-key and metadata contracts live in
//! `crate::common::foreign_keys`; this file keeps what MySQL cannot share
//! (`ON DELETE SET DEFAULT`) and the UUID-keyed `Complex`/`Post` reference.

#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::Role;
use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;

#[cfg(feature = "uuid")]
use uuid::Uuid;

#[cfg(not(feature = "uuid"))]
use crate::common::schema::sqlite::Post;
#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::{Complex, InsertComplex, InsertPost, Post, SelectPost};

/// Parent table for foreign key action tests
#[SQLiteTable]
pub struct FkParent {
    #[column(primary, autoincrement)]
    pub id: i32,
    pub name: String,
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

/// Kept for `crate::sqlite::seed`, which seeds through this cascade schema.
#[SQLiteTable]
pub struct FkCascade {
    #[column(primary, autoincrement)]
    pub id: i32,
    #[column(references = FkParent::id, on_delete = CASCADE)]
    pub parent_id: Option<i32>,
    pub value: String,
}

#[SQLiteTable]
pub struct CompositeFkParent {
    #[column(primary)]
    pub id_a: i32,
    #[column(primary)]
    pub id_b: i32,
    pub label: String,
}

#[SQLiteTable(FOREIGN_KEY(
    columns(parent_a, parent_b),
    references(CompositeFkParent, id_a, id_b),
    on_delete = "CASCADE",
    on_update = "CASCADE"
))]
pub struct CompositeFkChild {
    #[column(primary, autoincrement)]
    pub id: i32,
    pub parent_a: Option<i32>,
    pub parent_b: Option<i32>,
    pub value: String,
}

#[derive(SQLiteSchema)]
pub struct FkCascadeSchema {
    pub fk_parent: FkParent,
    pub fk_cascade: FkCascade,
}

#[derive(SQLiteSchema)]
pub struct CompositeFkSchema {
    pub composite_fk_parent: CompositeFkParent,
    pub composite_fk_child: CompositeFkChild,
}

#[derive(SQLiteSchema)]
pub struct FkSetDefaultSchema {
    pub fk_parent: FkParent,
    pub fk_set_default: FkSetDefault,
}

#[allow(dead_code)]
#[derive(Debug, SQLiteFromRow)]
struct ParentResult {
    id: i32,
    name: String,
}

#[allow(dead_code)]
#[derive(Debug, SQLiteFromRow)]
struct ChildDefaultResult {
    id: i32,
    parent_id: i32,
    value: String,
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
fn test_foreign_key_reference_sql() {
    let post_sql = Post::create_table_sql();
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
    // The FK reference uses backtick-quoted identifier
    assert!(
        post_sql.contains("`id`"),
        "Post table should reference id column. Got: {}",
        post_sql
    );

    // Note: The common Post schema doesn't define ON DELETE/ON UPDATE actions
    // Those are tested separately in the dedicated action tests above
}

#[drizzle::test]
fn test_set_default_sets_default_value(db: &mut TestDb<FkSetDefaultSchema>) {
    let FkSetDefaultSchema {
        fk_parent,
        fk_set_default,
    } = schema;

    // Insert default parent - we'll use its id as the default (note: using with_id to set specific id)

    db.insert(fk_parent)
        .values([InsertFkParent::new("DefaultParent").with_id(0)])
        .execute();

    // Insert parent that we'll delete

    db.insert(fk_parent)
        .values([InsertFkParent::new("Parent1")])
        .execute();

    // Get the non-default parent's ID
    let parents: Vec<ParentResult> = db
        .select(())
        .from(fk_parent)
        .r#where(eq(fk_parent.name, "Parent1"))
        .all();
    let parent_id = parents[0].id;

    // Insert child referencing parent (parent_id has DEFAULT = 0, but we override it)

    db.insert(fk_set_default)
        .values([InsertFkSetDefault::new("Child1").with_parent_id(parent_id)])
        .execute();

    // Verify child has parent_id = parent_id
    let children: Vec<ChildDefaultResult> = db.select(()).from(fk_set_default).all();
    assert_eq!(1, children.len());
    assert_eq!(parent_id, children[0].parent_id);

    // Delete the parent - should set child's parent_id to default (0)

    db.delete(fk_parent)
        .r#where(eq(fk_parent.id, parent_id))
        .execute();

    // Verify child's parent_id is now the default value (0)
    let children: Vec<ChildDefaultResult> = db.select(()).from(fk_set_default).all();
    assert_eq!(1, children.len(), "Child should still exist");
    assert_eq!(
        0, children[0].parent_id,
        "Parent ID should be default (0) after SET DEFAULT"
    );
}

#[cfg(feature = "uuid")]
#[derive(SQLiteSchema)]
pub struct ComplexPostSchema {
    pub complex: Complex,
    pub post: Post,
}

#[cfg(feature = "uuid")]
#[drizzle::test]
fn test_foreign_key_impl(db: &mut TestDb<ComplexPostSchema>) {
    let ComplexPostSchema { complex, post } = schema;

    let id = Uuid::new_v4();

    db.insert(complex)
        .values([InsertComplex::new("John", false, Role::User).with_id(id)])
        .execute();

    db.insert(post)
        .values([InsertPost::new("test", true).with_author_id(id)])
        .execute();

    let row: SelectPost = db
        .select(())
        .from(post)
        .r#where(eq(post.author_id, id))
        .get();

    assert_eq!(Some(id), row.author_id);
}
