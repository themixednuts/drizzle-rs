use drizzle::mysql::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, MySQLEnum)]
pub enum Role {
    #[default]
    Member,
    Admin,
}

#[MySQLTable(NAME = "test_users")]
pub struct User {
    #[column(PRIMARY, AUTO_INCREMENT)]
    pub id: u64,
    #[column(VARCHAR(255))]
    pub name: String,
    pub active: bool,
    #[column(ENUM)]
    pub role: Role,
    pub note: Option<String>,
    pub payload: Vec<u8>,
    pub balance: i64,
    pub score: f64,
}

#[MySQLTable(NAME = "test_posts")]
pub struct Post {
    #[column(PRIMARY, AUTO_INCREMENT)]
    pub id: u64,
    #[column(REFERENCES = User::id)]
    pub user_id: u64,
    #[column(VARCHAR(255))]
    pub title: String,
}

#[MySQLIndex(unique)]
pub struct UserNameIndex(User::name);

#[derive(MySQLSchema)]
pub struct TestSchema {
    pub users: User,
    pub users_name_index: UserNameIndex,
    pub posts: Post,
}
