//! Prepared-statement contracts exercised through mysql-sync.

use crate::common::schema::mysql::*;
use drizzle::core::expr::eq;
use drizzle::mysql::prelude::*;

macro_rules! user {
    ($name:expr) => {
        InsertUser::new($name, true, Role::Member, vec![], 0, 0.0).with_note(None::<String>)
    };
}

#[drizzle::test]
fn prepared_select_reuses_typed_parameters(db: &mut TestDb<TestSchema>) {
    let TestSchema { users, .. } = schema;
    db.insert(users)
        .values([user!("Alice"), user!("Bob"), user!("Charlie")])
        .execute();

    let name = users.name.placeholder("name");
    let prepared = db
        .select(())
        .from(users)
        .r#where(eq(users.name, name))
        .prepare()
        .into_owned();

    let alice: Vec<SelectUser> = prepared.all(drizzle_client!(), [name.bind("Alice")]);
    let bob: SelectUser = prepared.get(drizzle_client!(), [name.bind("Bob")]);
    assert_eq!(alice.len(), 1);
    assert_eq!(alice[0].name, "Alice");
    assert_eq!(bob.name, "Bob");
}

#[drizzle::test]
fn prepared_mutation_reports_affected_rows(db: &mut TestDb<TestSchema>) {
    let TestSchema { users, .. } = schema;
    let inserted = db.insert(users).value(user!("Alice")).execute();
    let id = inserted.last_insert_id().expect("AUTO_INCREMENT id");

    let name = users.name.placeholder("name");
    let user_id = users.id.placeholder("user_id");
    let prepared = db
        .update(users)
        .set(UpdateUser::default().with_name(name))
        .r#where(eq(users.id, user_id))
        .prepare()
        .into_owned();
    let result = prepared.execute(drizzle_client!(), [name.bind("Alicia"), user_id.bind(id)]);
    assert_eq!(result.affected_rows(), 1);

    let renamed: SelectUser = db.select(()).from(users).r#where(eq(users.id, id)).get();
    assert_eq!(renamed.name, "Alicia");
}
