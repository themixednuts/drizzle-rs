//! Auto-generated PostgreSQL schema from introspection
//!
//! Imported from the drizzle-kit snapshot stable/postgres/meta/0001_snapshot.json

use drizzle::postgres::prelude::*;

#[allow(non_camel_case_types)]
#[derive(PostgresEnum, Clone, Copy, Debug, Default, PartialEq)]
#[postgres_enum(schema = "auth")]
pub enum account_status {
    #[default]
    active,
    disabled,
}

#[allow(non_camel_case_types)]
#[derive(PostgresEnum, Clone, Copy, Debug, Default, PartialEq)]
pub enum role {
    #[default]
    admin,
    member,
    guest,
}

#[PostgresTable(schema = "auth")]
pub struct Accounts {
    #[column(serial, primary)]
    pub id: i32,
    #[column(identity(by_default, increment = 2, start = 100))]
    pub seq: i64,
    #[column(enum, default = "active")]
    pub status: account_status,
    #[column(references = Users::id, fk_name = "accounts_user_id_users_id_fk", on_delete = cascade)]
    pub user_id: i32,
}

#[PostgresTable(name = "auditLog")]
pub struct AuditLog {
    #[column(name = "createdAt", default = now())]
    pub created_at: chrono::NaiveDateTime,
    #[column(serial, primary)]
    pub id: i32,
    pub payload: Option<serde_json::Value>,
    #[column(default = "event")]
    pub type_: String,
    #[column(name = "userId", references = Users::id, fk_name = "auditLog_userId_users_id_fk", on_delete = set_null)]
    pub user_id: Option<i32>,
}

#[PostgresTable(primary_key(name = "post_tag_votes_pk"), foreign_key(columns(post_id, tag_id), references(PostTags, post_id, tag_id), name = "post_tag_votes_post_id_tag_id_post_tags_post_id_tag_id_fk", on_delete = "CASCADE"))]
pub struct PostTagVotes {
    #[column(primary)]
    pub post_id: i32,
    #[column(primary)]
    pub tag_id: i32,
    #[column(default = 1)]
    pub value: i32,
    #[column(primary, references = Users::id, fk_name = "post_tag_votes_voter_id_users_id_fk")]
    pub voter_id: i32,
}

#[PostgresTable(primary_key(name = "post_tags_post_id_tag_id_pk"))]
pub struct PostTags {
    #[column(primary, references = Posts::id, fk_name = "post_tags_post_id_posts_id_fk", on_delete = cascade)]
    pub post_id: i32,
    #[column(primary, references = Tags::id, fk_name = "post_tags_tag_fk", on_delete = restrict)]
    pub tag_id: i32,
}

#[PostgresTable(unique(columns(slug, author_id), name = "posts_slug_author_unique", nulls_not_distinct))]
pub struct Posts {
    #[column(references = Users::id, fk_name = "posts_author_id_users_id_fk", on_delete = cascade, on_update = cascade)]
    pub author_id: i32,
    #[column(references = Users::id, fk_name = "posts_editor_id_users_id_fk", on_delete = set_null)]
    pub editor_id: Option<i32>,
    #[column(serial, primary)]
    pub id: i32,
    pub published_at: Option<chrono::NaiveDateTime>,
    pub slug: String,
    pub title: String,
}

#[PostgresTable(unique(columns(name), name = "tags_name_unique"))]
pub struct Tags {
    #[column(serial, primary)]
    pub id: i32,
    pub name: String,
}

#[PostgresTable(unique(columns(email), name = "users_email_unique"), check(name = "balance_non_negative", expr = "\"users\".\"balance\" >= 0"))]
pub struct Users {
    #[column(default = 0)]
    pub balance: i32,
    pub birthday: Option<chrono::NaiveDate>,
    #[column(CHAR(2))]
    pub country_code: Option<String>,
    #[column(default = now())]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[column(VARCHAR(255))]
    pub email: String,
    #[column(default = gen_random_uuid())]
    pub external_id: uuid::Uuid,
    #[column(identity(always), primary)]
    pub id: i32,
    #[column(default = true)]
    pub is_active: bool,
    pub last_ip: Option<cidr::IpInet>,
    #[column(default = 1)]
    pub level: i16,
    #[column(default = "anon")]
    pub name: String,
    pub rating: Option<f64>,
    #[column(enum, default = "member")]
    pub role: role,
    pub scores: Option<Vec<i32>>,
    #[column(generated(stored, "lower(name)"))]
    pub search_name: Option<String>,
    #[column(default = "{}")]
    pub settings: serde_json::Value,
    #[column(default = "{}")]
    pub tags: Vec<String>,
    pub updated_at: Option<chrono::NaiveDateTime>,
}

#[PostgresIndex(where = "\"posts\".\"published_at\" is not null")]
pub struct PostsPublishedIdx(Posts::published_at);

#[PostgresIndex]
pub struct UsersCreatedIdx(Users::created_at);

#[PostgresIndex(unique)]
pub struct UsersExternalIdIdx(Users::external_id);

#[PostgresIndex]
pub struct UsersNameIdx(Users::name);

#[PostgresView(definition = "select \"id\", \"email\" from \"users\" where \"users\".\"is_active\" = true")]
pub struct ActiveUsers {
    pub email: String,
    pub id: i32,
}

#[PostgresView(materialized, definition = "select \"author_id\", count(*) as \"total\" from \"posts\" group by \"posts\".\"author_id\"")]
pub struct PostCounts {
    pub author_id: i32,
    pub total: i64,
}

#[derive(PostgresSchema)]
pub struct Schema {
    pub account_status: account_status,
    pub role: role,
    pub accounts: Accounts,
    pub audit_log: AuditLog,
    pub post_tag_votes: PostTagVotes,
    pub post_tags: PostTags,
    pub posts: Posts,
    pub tags: Tags,
    pub users: Users,
    pub posts_published_idx: PostsPublishedIdx,
    pub users_created_idx: UsersCreatedIdx,
    pub users_external_id_idx: UsersExternalIdIdx,
    pub users_name_idx: UsersNameIdx,
    pub active_users: ActiveUsers,
    pub post_counts: PostCounts,
}
