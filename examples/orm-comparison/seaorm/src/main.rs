use sea_orm::entity::prelude::*;
use sea_orm::{ConnectionTrait, Database, EntityTrait, QueryFilter, QueryOrder, Set};

mod user {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "users")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub name: String,
        pub email: Option<String>,
        pub age: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(has_many = "super::post::Entity")]
        Posts,
    }

    impl Related<super::post::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Posts.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

mod post {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "posts")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub title: String,
        pub content: Option<String>,
        pub author_id: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::user::Entity",
            from = "Column::AuthorId",
            to = "super::user::Column::Id"
        )]
        User,
    }

    impl Related<super::user::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::User.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

use post::Entity as Post;
use user::Entity as User;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::connect("sqlite::memory:").await?;
    db.execute_unprepared(include_str!("../../schema.sql")).await?;
    db.execute_unprepared(include_str!("../../seed.sql")).await?;

    println!("--- select ---");
    let rows = User::find()
        .filter(user::Column::Age.gt(25))
        .order_by_asc(user::Column::Name)
        .all(&db)
        .await?;
    for u in &rows {
        println!("{} ({})", u.name, u.age);
    }

    println!("--- insert ---");
    user::ActiveModel {
        name: Set("Sam".into()),
        email: Set(Some("sam@example.com".into())),
        age: Set(22),
        ..Default::default()
    }
    .insert(&db)
    .await?;

    println!("--- update ---");
    let mut am: user::ActiveModel = User::find_by_id(1).one(&db).await?.unwrap().into();
    am.age = Set(27);
    am.update(&db).await?;

    println!("--- join ---");
    let joined = User::find()
        .find_also_related(Post)
        .all(&db)
        .await?;
    for (u, post) in joined {
        let title = post.map(|p| p.title).unwrap_or_else(|| "(no post)".into());
        println!("{} | {}", u.name, title);
    }

    println!("--- relations ---");
    let users = User::find().find_with_related(Post).all(&db).await?;
    for (u, posts) in users {
        println!("{}: {} posts", u.name, posts.len());
    }

    Ok(())
}
