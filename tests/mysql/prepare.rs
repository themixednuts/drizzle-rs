//! MySQL prepared-statement execution metadata.

use crate::common::schema::mysql::*;
use drizzle::core::expr::eq;
use drizzle::mysql::prelude::*;

macro_rules! user {
    ($name:expr) => {
        InsertUser::new($name, true, Role::Member, vec![], 0, 0.0).with_note(None::<String>)
    };
}

#[drizzle::test]
fn prepared_mutation_reports_mysql_execution_metadata(db: &mut TestDb<TestSchema>) {
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
