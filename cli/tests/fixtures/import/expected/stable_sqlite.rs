//! Auto-generated SQLite schema from introspection
//!
//! Imported from the drizzle-kit snapshot stable/sqlite/meta/0001_snapshot.json

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
    #[column(name = "userId", references = Users::id, on_delete = set_null, fk_name = "auditLog_userId_users_id_fk")]
    pub user_id: Option<i64>,
}

#[SQLiteTable(primary_key(name = "post_tag_votes_post_id_tag_id_voter_id_pk"), foreign_key(columns(post_id, tag_id), references(PostTags, post_id, tag_id), on_delete = "cascade", name = "post_tag_votes_post_id_tag_id_post_tags_post_id_tag_id_fk"))]
pub struct PostTagVotes {
    #[column(primary)]
    pub post_id: i64,
    #[column(primary)]
    pub tag_id: i64,
    #[column(default = 1)]
    pub value: i64,
    #[column(primary, references = Users::id, fk_name = "post_tag_votes_voter_id_users_id_fk")]
    pub voter_id: i64,
}

#[SQLiteTable(primary_key(name = "post_tags_post_id_tag_id_pk"))]
pub struct PostTags {
    #[column(primary, references = Posts::id, on_delete = cascade, fk_name = "post_tags_post_id_posts_id_fk")]
    pub post_id: i64,
    #[column(primary, references = Tags::id, on_delete = restrict, fk_name = "post_tags_tag_fk")]
    pub tag_id: i64,
}

#[SQLiteTable]
pub struct Posts {
    #[column(references = Users::id, on_delete = cascade, on_update = cascade, fk_name = "posts_author_id_users_id_fk")]
    pub author_id: i64,
    pub body: Option<String>,
    #[column(references = Users::id, on_delete = set_null, fk_name = "posts_editor_id_users_id_fk")]
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
    pub name: String,
}

#[SQLiteTable]
pub struct Users {
    #[column(check = "\"users\".\"age\" >= 0")]
    pub age: Option<i64>,
    pub avatar: Option<Vec<u8>>,
    #[column(default = (unixepoch()))]
    pub created_at: i64,
    #[column(default = "anon")]
    pub display_name: String,
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

#[SQLiteIndex(unique)]
pub struct PostsSlugAuthorUnique(Posts::slug, Posts::author_id);

#[SQLiteIndex(unique)]
pub struct TagsNameUnique(Tags::name);

#[SQLiteIndex]
pub struct UsersDisplayNameIdx(Users::display_name);

#[SQLiteIndex(unique)]
pub struct UsersEmailUnique(Users::email);

#[SQLiteView(definition = "select \"id\", \"title\" from \"posts\" where \"posts\".\"published_at\" is not null")]
pub struct PublishedPosts {
    pub id: i64,
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
    pub posts_slug_author_unique: PostsSlugAuthorUnique,
    pub tags_name_unique: TagsNameUnique,
    pub users_display_name_idx: UsersDisplayNameIdx,
    pub users_email_unique: UsersEmailUnique,
    pub published_posts: PublishedPosts,
}
