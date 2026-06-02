mod schema;

use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use schema::{posts, users};

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = users)]
struct User {
    id: i32,
    name: String,
    email: Option<String>,
    age: i32,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct NewUser<'a> {
    name: &'a str,
    email: Option<&'a str>,
    age: i32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = SqliteConnection::establish(":memory:")?;
    conn.batch_execute(include_str!("../../schema.sql"))?;
    conn.batch_execute(include_str!("../../seed.sql"))?;

    println!("--- select ---");
    let rows = users::table
        .filter(users::age.gt(25))
        .order(users::name.asc())
        .select(User::as_select())
        .load(&mut conn)?;
    for u in &rows {
        println!("{} ({})", u.name, u.age);
    }

    println!("--- insert ---");
    diesel::insert_into(users::table)
        .values(&NewUser {
            name: "Sam",
            email: Some("sam@example.com"),
            age: 22,
        })
        .execute(&mut conn)?;

    println!("--- update ---");
    diesel::update(users::table.find(1))
        .set(users::age.eq(27))
        .execute(&mut conn)?;

    println!("--- join ---");
    type JoinRow = (String, Option<String>);
    let joined: Vec<JoinRow> = users::table
        .left_join(posts::table.on(posts::author_id.eq(users::id)))
        .select((users::name, posts::title.nullable()))
        .load(&mut conn)?;
    for (name, title) in joined {
        println!("{} | {}", name, title.as_deref().unwrap_or("(no post)"));
    }

    println!("--- relations ---");
    let all_users = users::table.load::<User>(&mut conn)?;
    for u in all_users {
        let count = posts::table
            .filter(posts::author_id.eq(u.id))
            .count()
            .get_result::<i64>(&mut conn)?;
        println!("{}: {} posts", u.name, count);
    }

    Ok(())
}
