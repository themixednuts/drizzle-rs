#[derive(Debug, toasty::Model)]
struct User {
    #[key]
    #[auto]
    id: u64,
    name: String,
    email: Option<String>,
    age: i64,
    #[has_many(pair = author)]
    posts: toasty::HasMany<Post>,
}

#[derive(Debug, toasty::Model)]
struct Post {
    #[key]
    #[auto]
    id: u64,
    title: String,
    content: Option<String>,
    #[index]
    author_id: u64,
    #[belongs_to(key = author_id, references = id)]
    author: toasty::BelongsTo<User>,
}

async fn seed(db: &mut toasty::Db) -> toasty::Result<()> {
    toasty::create!(User {
        name: "Alex Smith",
        email: "alex@example.com",
        age: 26,
        posts: [
            { title: "Hello", content: "first post" },
            { title: "World", content: "second post" },
        ],
    })
    .exec(db)
    .await?;

    toasty::create!(User {
        name: "Jordan Lee",
        email: "jordan@example.com",
        age: 30,
    })
    .exec(db)
    .await?;

    toasty::create!(User {
        name: "Alice",
        email: "alice@example.com",
        age: 28,
    })
    .exec(db)
    .await?;

    toasty::create!(User {
        name: "Bob",
        email: "bob@example.com",
        age: 32,
    })
    .exec(db)
    .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> toasty::Result<()> {
    let mut db = toasty::Db::builder()
        .models(toasty::models!(crate::*))
        .connect("sqlite::memory:")
        .await?;
    db.push_schema().await?;
    seed(&mut db).await?;

    println!("--- select ---");
    let rows = User::filter(User::fields().age().gt(25))
        .order_by(User::fields().name().asc())
        .exec(&mut db)
        .await?;
    for u in &rows {
        println!("{} ({})", u.name, u.age);
    }

    println!("--- insert ---");
    toasty::create!(User {
        name: "Sam",
        email: "sam@example.com",
        age: 22,
    })
    .exec(&mut db)
    .await?;

    println!("--- update ---");
    let mut user = User::get_by_id(&mut db, &1u64).await?;
    user.update().age(27).exec(&mut db).await?;

    println!("--- join ---");
    let users = User::all()
        .include(User::fields().posts())
        .exec(&mut db)
        .await?;
    for u in &users {
        let posts = u.posts.get();
        if posts.is_empty() {
            println!("{} | (no post)", u.name);
        } else {
            for p in posts {
                println!("{} | {}", u.name, p.title);
            }
        }
    }

    println!("--- relations ---");
    let loaded = User::filter_by_id(1u64)
        .include(User::fields().posts())
        .get(&mut db)
        .await?;
    println!("{}: {} posts", loaded.name, loaded.posts.get().len());

    Ok(())
}
