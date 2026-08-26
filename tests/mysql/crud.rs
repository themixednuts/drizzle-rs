//! Cross-driver CRUD and selection contracts exercised through mysql-sync.

use crate::common::schema::mysql::*;
use drizzle::core::expr::{count, eq};

macro_rules! user {
    ($name:expr, $role:expr) => {
        InsertUser::new($name, true, $role, vec![1, 2, 3], -42, 9.5).with_note(None::<String>)
    };
}

#[drizzle::test]
fn insert_select_update_and_delete(db: &mut TestDb<TestSchema>) {
    let TestSchema { users, posts, .. } = schema;

    let inserted = db
        .insert(users)
        .value(user!("Alice", Role::Admin))
        .execute();
    let alice_id = inserted.last_insert_id().expect("AUTO_INCREMENT id");
    assert_eq!(inserted.affected_rows(), 1);

    db.insert(users).value(user!("Bob", Role::Member)).execute();
    db.insert(posts)
        .value(InsertPost::new(alice_id, "Hello"))
        .execute();

    let alice: SelectUser = db
        .select(())
        .from(users)
        .r#where(eq(users.id, alice_id))
        .get();
    assert_eq!(alice.name, "Alice");
    assert_eq!(alice.role, Role::Admin);
    assert_eq!(alice.note, None);
    assert_eq!(alice.payload, vec![1, 2, 3]);
    assert_eq!(alice.balance, -42);
    assert_eq!(alice.score, 9.5);

    let updated = db
        .update(users)
        .set(UpdateUser::default().with_note("updated"))
        .r#where(eq(users.id, alice_id))
        .execute();
    assert_eq!(updated.affected_rows(), 1);

    let updated: SelectUser = db
        .select(())
        .from(users)
        .r#where(eq(users.id, alice_id))
        .get();
    assert_eq!(updated.note.as_deref(), Some("updated"));

    let deleted = db
        .delete(posts)
        .r#where(eq(posts.user_id, alice_id))
        .execute();
    assert_eq!(deleted.affected_rows(), 1);
    let user_count: i64 = db.select(count(users.id)).from(users).get();
    assert_eq!(user_count, 2);
}

#[drizzle::test]
fn detached_queries_and_set_operations(db: &mut TestDb<TestSchema>) {
    let TestSchema { users, .. } = schema;
    db.insert(users)
        .values([user!("Alice", Role::Admin), user!("Bob", Role::Member)])
        .execute();

    let direct = db.select(()).from(users).detach();
    let selected: Vec<SelectUser> = db.all(direct);
    assert_eq!(selected.len(), 2);

    let bob_ids = db
        .select(users.id)
        .from(users)
        .r#where(eq(users.name, "Bob"))
        .detach();
    let ids: Vec<u64> = db
        .select(users.id)
        .from(users)
        .r#where(eq(users.name, "Alice"))
        .union_all(bob_ids)
        .all();
    assert_eq!(ids.len(), 2);
}
