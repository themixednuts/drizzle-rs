//! Auto-generated SQLite schema from introspection
//!
//! Imported from the drizzle-kit snapshot beta/sqlite/20261002162543_full/snapshot.json

use drizzle::sqlite::prelude::*;

#[SQLiteTable(name = "auditLog")]
pub struct AuditLog {
    #[column(name = "createdAt")]
    pub created_at: i64,
    #[column(primary)]
    pub id: i64,
    pub payload: Option<String>,
    #[column(default = "event")]
    pub type_: String,
    #[column(name = "userId", references = Users::id, on_delete = set_null)]
    pub user_id: Option<i64>,
}

#[SQLiteTable(foreign_key(columns(post_id, tag_id), references(PostTags, post_id, tag_id), on_delete = "cascade"))]
pub struct PostTagVotes {
    #[column(primary)]
    pub post_id: i64,
    #[column(primary)]
    pub tag_id: i64,
    #[column(default = 1)]
    pub value: i64,
    #[column(primary, references = Users::id)]
    pub voter_id: i64,
}

#[SQLiteTable]
pub struct PostTags {
    #[column(primary, references = Posts::id, on_delete = cascade)]
    pub post_id: i64,
    #[column(primary, references = Tags::id, on_delete = restrict)]
    pub tag_id: i64,
}

#[SQLiteTable(unique(columns(slug, author_id), name = "posts_slug_author_unique"))]
pub struct Posts {
    #[column(references = Users::id, on_delete = cascade, on_update = cascade)]
    pub author_id: i64,
    pub body: Option<String>,
    #[column(references = Users::id, on_delete = set_null)]
    pub editor_id: Option<i64>,
    #[column(primary)]
    pub id: i64,
    pub published_at: Option<i64>,
    pub slug: String,
    pub title: String,
}

#[SQLiteTable]
pub struct Tags {
    #[column(primary)]
    pub id: i64,
    #[column(unique)]
    pub name: String,
}

#[SQLiteTable]
pub struct Users {
    #[column(check = "\"age\" >= 0")]
    pub age: Option<i64>,
    pub avatar: Option<Vec<u8>>,
    #[column(default = (unixepoch()))]
    pub created_at: i64,
    #[column(default = "anon")]
    pub display_name: String,
    #[column(unique)]
    pub email: String,
    #[column(generated(virtual, "(substr(email, instr(email, '@') + 1))"))]
    pub email_domain: Option<String>,
    #[column(primary, autoincrement)]
    pub id: i64,
    #[column(default = false)]
    pub is_admin: bool,
    pub meta: Option<String>,
    #[column(default = "member")]
    pub role: String,
    #[column(default = 0.0)]
    pub score: Option<f64>,
}

#[SQLiteIndex(unique)]
pub struct PostsAuthorTitleIdx(Posts::author_id, Posts::title);

#[SQLiteIndex(where = "\"posts\".\"published_at\" is not null")]
pub struct PostsPublishedIdx(Posts::published_at);

#[SQLiteIndex]
pub struct UsersDisplayNameIdx(Users::display_name);

#[SQLiteView(definition = "select \"id\", \"title\" from \"posts\" where (\"posts\".\"published_at\" is not null)")]
pub struct PublishedPosts {
    pub id: Option<i64>,
    pub title: String,
}

#[derive(SQLiteSchema)]
pub struct Schema {
    pub audit_log: AuditLog,
    pub post_tag_votes: PostTagVotes,
    pub post_tags: PostTags,
    pub posts: Posts,
    pub tags: Tags,
    pub users: Users,
    pub posts_author_title_idx: PostsAuthorTitleIdx,
    pub posts_published_idx: PostsPublishedIdx,
    pub users_display_name_idx: UsersDisplayNameIdx,
    pub published_posts: PublishedPosts,
}
