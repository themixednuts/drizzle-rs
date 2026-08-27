//! MySQL value, mutation-result, and detached-builder contracts.

use crate::common::schema::mysql::*;
use drizzle::core::{
    asc,
    expr::{concat, count, eq},
};

#[drizzle::test]
fn mysql_values_and_mutation_metadata_round_trip(db: &mut TestDb<TestSchema>) {
    let TestSchema { users, posts, .. } = schema;

    let inserted = db
        .insert(users)
        .value(
            InsertUser::new("Alice", true, Role::Admin, vec![1, 2, 3], -42, 9.5)
                .with_note(None::<String>),
        )
        .execute();
    let alice_id = inserted.last_insert_id().expect("AUTO_INCREMENT id");
    assert_eq!(inserted.affected_rows(), 1);

    db.insert(users)
        .value(
            InsertUser::new("Bob", true, Role::Member, vec![1, 2, 3], -42, 9.5)
                .with_note(None::<String>),
        )
        .execute();
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
fn detached_builder_executes_through_the_mysql_driver(db: &mut TestDb<TestSchema>) {
    let TestSchema { users, .. } = schema;
    db.insert(users)
        .value(
            InsertUser::new("Alice", true, Role::Admin, vec![1, 2, 3], -42, 9.5)
                .with_note(None::<String>),
        )
        .execute();

    let detached = db.select(()).from(users).detach();
    let selected: Vec<SelectUser> = db.all(detached);
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].name, "Alice");
}

#[drizzle::test]
fn mysql_executes_concat_and_standalone_offset(db: &mut TestDb<TestSchema>) {
    let TestSchema { users, .. } = schema;
    db.insert(users)
        .values([
            InsertUser::new("Alice", true, Role::Admin, vec![], 0, 0.0).with_note(None::<String>),
            InsertUser::new("Bob", true, Role::Member, vec![], 0, 0.0).with_note(None::<String>),
        ])
        .execute();

    let labels: Vec<String> = db
        .select(concat(users.name, "!"))
        .from(users)
        .order_by(asc(users.id))
        .offset(1)
        .all();

    assert_eq!(labels, ["Bob!"]);
}
