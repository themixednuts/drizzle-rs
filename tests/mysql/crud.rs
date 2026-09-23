//! MySQL value, mutation-result, and detached-builder contracts.

use crate::common::schema::mysql::*;
use drizzle::core::{
    asc,
    expr::{concat, count, eq},
};
use drizzle::mysql::prelude::*;

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
fn partial_insert_select_round_trips(db: &mut TestDb<TestSchema>) {
    let TestSchema { users, posts, .. } = schema;
    db.insert(users)
        .values([
            InsertUser::new("Alice", true, Role::Admin, vec![], 0, 0.0),
            InsertUser::new("Bob", true, Role::Member, vec![], 0, 0.0),
        ])
        .execute();

    let selected = db.select((users.id, users.name)).from(users).detach();
    let inserted = db
        .insert(posts)
        .columns((posts.user_id, posts.title))
        .select(selected)
        .execute();
    assert_eq!(inserted.affected_rows(), 2);

    let rows: Vec<SelectPost> = db.select(()).from(posts).order_by(asc(posts.user_id)).all();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].title, "Alice");
    assert_eq!(rows[1].title, "Bob");
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

#[MySQLTable(NAME = "insert_optional_rows")]
struct OptionalRows {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    #[column(VARCHAR(32))]
    name: String,
    #[column(VARCHAR(32))]
    nickname: Option<String>,
    #[column(VARCHAR(32), DEFAULT = "guest")]
    role: Option<String>,
}

#[derive(MySQLSchema)]
struct InsertRowsSchema {
    optional_rows: OptionalRows,
}

#[drizzle::test]
fn multi_row_insert_lines_up_rows_that_omit_different_columns(db: &mut TestDb<InsertRowsSchema>) {
    let InsertRowsSchema { optional_rows } = schema;

    // Both rows set `nickname` and `role`, but `None` leaves a column to its
    // default without changing the row's type, so the rows name different
    // columns. Row 0's column list used to serve every row, which put row 1's
    // role into `nickname`.
    let insert = db.insert(optional_rows).values([
        InsertOptionalRows::new("a")
            .with_nickname(Some(String::from("A")))
            .with_role(None::<String>),
        InsertOptionalRows::new("b")
            .with_nickname(None::<String>)
            .with_role(Some(String::from("admin"))),
    ]);
    let sql = insert.to_sql().sql();
    assert!(
        sql.ends_with("(`name`, `nickname`, `role`) VALUES (?, ?, DEFAULT), (?, DEFAULT, ?)"),
        "{sql}"
    );
    insert.execute();

    let stored: Vec<SelectOptionalRows> = db
        .select(())
        .from(optional_rows)
        .order_by(asc(optional_rows.id))
        .all();
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[0].nickname.as_deref(), Some("A"));
    assert_eq!(stored[0].role.as_deref(), Some("guest"));
    assert_eq!(stored[1].nickname, None);
    assert_eq!(stored[1].role.as_deref(), Some("admin"));
}
