use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Executor, FromRow};

#[derive(Debug, FromRow)]
struct User {
    id: i64,
    name: String,
    email: Option<String>,
    age: i64,
}

#[derive(Debug, FromRow)]
struct UserPost {
    name: String,
    post_title: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await?;
    pool.execute(include_str!("../../schema.sql")).await?;
    pool.execute(include_str!("../../seed.sql")).await?;

    println!("--- select ---");
    let rows = sqlx::query_as::<_, User>(
        "SELECT id, name, email, age FROM users WHERE age > ? ORDER BY name",
    )
    .bind(25i64)
    .fetch_all(&pool)
    .await?;
    for u in &rows {
        println!("{} ({})", u.name, u.age);
    }

    println!("--- insert ---");
    sqlx::query("INSERT INTO users (name, email, age) VALUES (?, ?, ?)")
        .bind("Sam")
        .bind("sam@example.com")
        .bind(22i64)
        .execute(&pool)
        .await?;

    println!("--- update ---");
    sqlx::query("UPDATE users SET age = ? WHERE id = ?")
        .bind(27i64)
        .bind(1i64)
        .execute(&pool)
        .await?;

    println!("--- join ---");
    let joined = sqlx::query_as::<_, UserPost>(
        r#"
        SELECT u.name AS name, p.title AS post_title
        FROM users u
        LEFT JOIN posts p ON p.author_id = u.id
        "#,
    )
    .fetch_all(&pool)
    .await?;
    for row in joined {
        println!(
            "{} | {}",
            row.name,
            row.post_title.as_deref().unwrap_or("(no post)")
        );
    }

    println!("--- relations ---");
    let users = sqlx::query_as::<_, User>("SELECT id, name, email, age FROM users ORDER BY id")
        .fetch_all(&pool)
        .await?;
    for u in users {
        let posts = sqlx::query("SELECT title FROM posts WHERE author_id = ?")
            .bind(u.id)
            .fetch_all(&pool)
            .await?;
        println!("{}: {} posts", u.name, posts.len());
    }

    Ok(())
}
