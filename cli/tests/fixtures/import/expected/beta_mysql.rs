//! Auto-generated MySQL schema from introspection
//!
//! Imported from the drizzle-kit snapshot beta/mysql/20261002162552_full/snapshot.json

use drizzle::mysql::prelude::*;

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, MySQLEnum)]
pub enum UsersRoleEnum {
    admin,
    member,
}

#[MySQLTable(
    NAME = "auditLog",
    FOREIGN_KEY(columns(user_id), references(Users, id), name = "auditLog_userId_users_id_fkey", on_delete = "SET NULL", on_update = "NO ACTION"),
)]
pub struct AuditLog {
    #[column(NAME = "id", BIGINT_UNSIGNED, PRIMARY, AUTO_INCREMENT)]
    pub id: u64,
    #[column(NAME = "userId", BIGINT_UNSIGNED)]
    pub user_id: Option<u64>,
    #[column(NAME = "createdAt", TIMESTAMP, DEFAULT = (now()))]
    pub created_at: String,
    #[column(NAME = "type", VARCHAR(32), DEFAULT = "event")]
    pub type_: String,
    #[column(NAME = "payload", JSON)]
    pub payload: Option<String>,
}

#[MySQLTable(
    NAME = "post_tags",
    FOREIGN_KEY(columns(post_id), references(Posts, id), name = "post_tags_post_id_posts_id_fkey", on_delete = "CASCADE", on_update = "NO ACTION"),
    FOREIGN_KEY(columns(tag_id), references(Tags, id), name = "post_tags_tag_fk", on_delete = "RESTRICT", on_update = "NO ACTION"),
)]
pub struct PostTags {
    #[column(NAME = "post_id", BIGINT_UNSIGNED, PRIMARY)]
    pub post_id: u64,
    #[column(NAME = "tag_id", INT, PRIMARY)]
    pub tag_id: i32,
}

#[MySQLTable(
    NAME = "posts",
    FOREIGN_KEY(columns(author_id), references(Users, id), name = "posts_author_id_users_id_fkey", on_delete = "CASCADE", on_update = "CASCADE"),
)]
pub struct Posts {
    #[column(NAME = "id", BIGINT_UNSIGNED, PRIMARY, AUTO_INCREMENT)]
    pub id: u64,
    #[column(NAME = "author_id", BIGINT_UNSIGNED)]
    pub author_id: u64,
    #[column(NAME = "title", VARCHAR(200))]
    pub title: String,
    #[column(NAME = "slug", VARCHAR(200))]
    pub slug: String,
    #[column(NAME = "body", TEXT)]
    pub body: Option<String>,
}

#[MySQLTable(
    NAME = "tags",
)]
pub struct Tags {
    #[column(NAME = "id", INT, PRIMARY, AUTO_INCREMENT)]
    pub id: i32,
    #[column(NAME = "name", VARCHAR(64))]
    pub name: String,
}

#[MySQLTable(
    NAME = "users",
    CHECK(name = "balance_non_negative", expr = "`users`.`balance` >= 0"),
)]
pub struct Users {
    #[column(NAME = "id", BIGINT_UNSIGNED, PRIMARY, AUTO_INCREMENT)]
    pub id: u64,
    #[column(NAME = "email", VARCHAR(255))]
    pub email: String,
    #[column(NAME = "name", VARCHAR(100), DEFAULT = "anon")]
    pub name: String,
    #[column(NAME = "role", ENUM, DEFAULT = "member")]
    pub role: UsersRoleEnum,
    #[column(NAME = "balance", DECIMAL(10, 2), DEFAULT = (0.00))]
    pub balance: String,
    #[column(NAME = "rating", DOUBLE)]
    pub rating: Option<f64>,
    #[column(NAME = "level", TINYINT, DEFAULT = 1)]
    pub level: i8,
    #[column(NAME = "settings", JSON)]
    pub settings: Option<String>,
    #[column(NAME = "is_active", BOOLEAN, DEFAULT = TRUE)]
    pub is_active: bool,
    #[column(NAME = "created_at", TIMESTAMP, DEFAULT = (now()))]
    pub created_at: String,
    #[column(NAME = "updated_at", TIMESTAMP, ON_UPDATE = "CURRENT_TIMESTAMP")]
    pub updated_at: Option<String>,
    #[column(NAME = "last_seen", DATETIME)]
    pub last_seen: Option<String>,
    #[column(NAME = "name_lower", VARCHAR(100), generated(STORED, "lower(name)"))]
    pub name_lower: Option<String>,
}

#[MySQLIndex(unique)]
pub struct NameUnique(Tags::name);

#[MySQLIndex(unique)]
pub struct EmailUnique(Users::email);

#[MySQLIndex(unique)]
pub struct PostsSlugAuthorUnique(Posts::slug, Posts::author_id);

#[MySQLIndex(unique)]
pub struct PostsAuthorTitleIdx(Posts::author_id, Posts::title);

#[MySQLIndex]
pub struct UsersNameIdx(Users::name);

#[MySQLView(
    NAME = "active_users",
    DEFINITION = "select `id`, `email` from `users` where `users`.`is_active` = true",
    ALGORITHM = "undefined",
    SQL_SECURITY = "definer",
)]
pub struct ActiveUsers {
    #[column(NAME = "id", BIGINT_UNSIGNED)]
    pub id: u64,
    #[column(NAME = "email", VARCHAR(255))]
    pub email: String,
}

#[derive(MySQLSchema)]
pub struct Schema {
    pub audit_log: AuditLog,
    pub post_tags: PostTags,
    pub posts: Posts,
    pub tags: Tags,
    pub users: Users,
    pub name_unique: NameUnique,
    pub email_unique: EmailUnique,
    pub posts_slug_author_unique: PostsSlugAuthorUnique,
    pub posts_author_title_idx: PostsAuthorTitleIdx,
    pub users_name_idx: UsersNameIdx,
    pub active_users: ActiveUsers,
}
