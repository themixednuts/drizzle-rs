//! JOIN decoding contracts shared by the relational drivers.

use crate::common::schema::mysql::*;
use drizzle::core::expr::eq;

#[drizzle::test]
fn select_star_decodes_every_joined_table(db: &mut TestDb<TestSchema>) {
    let TestSchema { users, posts, .. } = schema;
    let inserted = db
        .insert(users)
        .value(
            InsertUser::new("Alice", true, Role::Admin, vec![1, 2, 3], -42, 9.5)
                .with_note(None::<String>),
        )
        .execute();
    let user_id = inserted.last_insert_id().expect("AUTO_INCREMENT id");
    db.insert(posts)
        .value(InsertPost::new(user_id, "Hello"))
        .execute();

    let joined: Vec<(SelectUser, SelectPost)> = db
        .select(())
        .from(users)
        .inner_join((posts, eq(posts.user_id, users.id)))
        .all();

    assert_eq!(joined.len(), 1);
    assert_eq!(joined[0].0.name, "Alice");
    assert_eq!(joined[0].1.title, "Hello");
}
