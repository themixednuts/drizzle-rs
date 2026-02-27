#![cfg(all(
    any(feature = "postgres-sync", feature = "tokio-postgres"),
    feature = "query"
))]

use drizzle::core::expr::{eq, gt};
use drizzle::core::{asc, desc};
use drizzle::postgres::prelude::*;
use drizzle_macros::postgres_test;

// =============================================================================
// Schema: QUser -> QPost -> QComment -> QReply (with self-referential QUser.invited_by)
// =============================================================================

#[PostgresTable]
struct QUser {
    #[column(serial, primary)]
    id: i32,
    name: String,
    #[column(references = QUser::id)]
    invited_by: Option<i32>,
}

#[PostgresTable]
struct QPost {
    #[column(serial, primary)]
    id: i32,
    content: String,
    #[column(references = QUser::id)]
    author_id: i32,
}

#[PostgresTable]
struct QComment {
    #[column(serial, primary)]
    id: i32,
    body: String,
    #[column(references = QPost::id)]
    post_id: i32,
}

#[PostgresTable]
struct QReply {
    #[column(serial, primary)]
    id: i32,
    text: String,
    #[column(references = QComment::id)]
    comment_id: i32,
}

// -- Schemas for different test scenarios --

#[derive(PostgresSchema)]
struct QUserPostSchema {
    q_user: QUser,
    q_post: QPost,
}

#[derive(PostgresSchema)]
struct QFullSchema {
    q_user: QUser,
    q_post: QPost,
    q_comment: QComment,
}

#[derive(PostgresSchema)]
struct QDeepSchema {
    q_user: QUser,
    q_post: QPost,
    q_comment: QComment,
    q_reply: QReply,
}

// =============================================================================
// Tests
// =============================================================================

// -- Basic find_many without relations --
postgres_test!(query_find_many_no_relations, QUserPostSchema, {
    let QUserPostSchema { q_user, q_post: _ } = schema;

    drizzle_exec!(
        db.insert(q_user)
            .values([
                InsertQUser::new("Alice"),
                InsertQUser::new("Bob"),
            ])
            => execute
    );

    let users = drizzle_exec!(db.query(q_user).find_many());
    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "Alice");
    assert_eq!(users[1].name, "Bob");
});

// -- find_first --
postgres_test!(query_find_first, QUserPostSchema, {
    let QUserPostSchema { q_user, q_post: _ } = schema;

    drizzle_exec!(
        db.insert(q_user)
            .values([
                InsertQUser::new("Alice"),
                InsertQUser::new("Bob"),
            ])
            => execute
    );

    let user = drizzle_exec!(db.query(q_user).find_first());
    assert!(user.is_some());
    assert_eq!(user.unwrap().name, "Alice");
});

// -- find_first returns None on empty table --
postgres_test!(query_find_first_empty, QUserPostSchema, {
    let QUserPostSchema { q_user, q_post: _ } = schema;

    let user = drizzle_exec!(db.query(q_user).find_first());
    assert!(user.is_none());
});

// -- with limit --
postgres_test!(query_with_limit, QUserPostSchema, {
    let QUserPostSchema { q_user, q_post: _ } = schema;

    drizzle_exec!(
        db.insert(q_user)
            .values([
                InsertQUser::new("Alice"),
                InsertQUser::new("Bob"),
                InsertQUser::new("Charlie"),
            ])
            => execute
    );

    let users = drizzle_exec!(db.query(q_user).limit(2).find_many());
    assert_eq!(users.len(), 2);
});

// -- Reverse relation: User -> Posts (Many) --
postgres_test!(query_reverse_relation_many, QUserPostSchema, {
    let QUserPostSchema { q_user, q_post } = schema;

    // Insert users
    drizzle_exec!(
        db.insert(q_user)
            .values([
                InsertQUser::new("Alice"),
                InsertQUser::new("Bob"),
            ])
            => execute
    );

    let all_users: Vec<SelectQUser> = drizzle_exec!(db.select(()).from(q_user) => all);
    let alice_id = all_users.iter().find(|u| u.name == "Alice").unwrap().id;
    let bob_id = all_users.iter().find(|u| u.name == "Bob").unwrap().id;

    // Insert posts
    drizzle_exec!(
        db.insert(q_post)
            .values([
                InsertQPost::new("Alice Post 1", alice_id),
                InsertQPost::new("Alice Post 2", alice_id),
                InsertQPost::new("Bob Post 1", bob_id),
            ])
            => execute
    );

    // Query users with their posts
    let users = drizzle_exec!(db.query(q_user).with(q_user.q_posts()).find_many());

    assert_eq!(users.len(), 2);

    // Alice has 2 posts
    let alice = users.iter().find(|u| u.name == "Alice").unwrap();
    assert_eq!(alice.q_posts().len(), 2);
    assert_eq!(alice.q_posts()[0].content, "Alice Post 1");
    assert_eq!(alice.q_posts()[1].content, "Alice Post 2");

    // Bob has 1 post
    let bob = users.iter().find(|u| u.name == "Bob").unwrap();
    assert_eq!(bob.q_posts().len(), 1);
    assert_eq!(bob.q_posts()[0].content, "Bob Post 1");
});

// -- Forward relation: Post -> Author (One) --
postgres_test!(query_forward_relation_one, QUserPostSchema, {
    let QUserPostSchema { q_user, q_post } = schema;

    drizzle_exec!(
        db.insert(q_user)
            .values([InsertQUser::new("Alice")])
            => execute
    );

    let all_users: Vec<SelectQUser> = drizzle_exec!(db.select(()).from(q_user) => all);
    let alice_id = all_users[0].id;

    drizzle_exec!(
        db.insert(q_post)
            .values([InsertQPost::new("Hello World", alice_id)])
            => execute
    );

    // Query posts with their author
    let posts = drizzle_exec!(db.query(q_post).with(q_post.author()).find_many());

    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0].content, "Hello World");
    assert_eq!(posts[0].author().name, "Alice");
});

// -- Forward relation: OptionalOne (nullable FK, self-referential) --
postgres_test!(query_forward_optional_one, QUserPostSchema, {
    let QUserPostSchema { q_user, q_post: _ } = schema;

    drizzle_exec!(
        db.insert(q_user)
            .values([InsertQUser::new("Alice")])
            => execute
    );

    let all_users: Vec<SelectQUser> = drizzle_exec!(db.select(()).from(q_user) => all);
    let alice_id = all_users[0].id;

    // Bob was invited by Alice
    drizzle_exec!(
        db.insert(q_user)
            .values([InsertQUser::new("Bob").with_invited_by(alice_id)])
            => execute
    );

    // Query users with their inviter
    let users = drizzle_exec!(db.query(q_user).with(q_user.invited_by()).find_many());

    assert_eq!(users.len(), 2);

    // Alice has no inviter
    let alice = users.iter().find(|u| u.name == "Alice").unwrap();
    assert!(alice.invited_by().is_none());

    // Bob was invited by Alice
    let bob = users.iter().find(|u| u.name == "Bob").unwrap();
    assert!(bob.invited_by().is_some());
    assert_eq!(bob.invited_by().as_ref().unwrap().name, "Alice");
});

// -- Nested relations: User -> Posts -> Comments --
postgres_test!(query_nested_relations, QFullSchema, {
    let QFullSchema {
        q_user,
        q_post,
        q_comment,
    } = schema;

    // Insert user
    drizzle_exec!(
        db.insert(q_user)
            .values([InsertQUser::new("Alice")])
            => execute
    );

    let all_users: Vec<SelectQUser> = drizzle_exec!(db.select(()).from(q_user) => all);
    let alice_id = all_users[0].id;

    // Insert posts
    drizzle_exec!(
        db.insert(q_post)
            .values([
                InsertQPost::new("Post 1", alice_id),
                InsertQPost::new("Post 2", alice_id),
            ])
            => execute
    );

    let all_posts: Vec<SelectQPost> = drizzle_exec!(db.select(()).from(q_post) => all);
    let post1_id = all_posts.iter().find(|p| p.content == "Post 1").unwrap().id;
    let post2_id = all_posts.iter().find(|p| p.content == "Post 2").unwrap().id;

    // Insert comments
    drizzle_exec!(
        db.insert(q_comment)
            .values([
                InsertQComment::new("Comment on P1-A", post1_id),
                InsertQComment::new("Comment on P1-B", post1_id),
                InsertQComment::new("Comment on P2", post2_id),
            ])
            => execute
    );

    // Query: users -> posts -> comments
    let users = drizzle_exec!(
        db.query(q_user)
            .with(q_user.q_posts().with(q_post.q_comments()))
            .find_many()
    );

    assert_eq!(users.len(), 1);
    let alice = &users[0];
    assert_eq!(alice.name, "Alice");
    assert_eq!(alice.q_posts().len(), 2);

    // Find post 1 and check its comments
    let p1 = alice
        .q_posts()
        .iter()
        .find(|p| p.content == "Post 1")
        .unwrap();
    assert_eq!(p1.q_comments().len(), 2);

    let p2 = alice
        .q_posts()
        .iter()
        .find(|p| p.content == "Post 2")
        .unwrap();
    assert_eq!(p2.q_comments().len(), 1);
    assert_eq!(p2.q_comments()[0].body, "Comment on P2");
});

// -- Multiple relations: User with posts AND invited_by --
postgres_test!(query_multiple_relations, QUserPostSchema, {
    let QUserPostSchema { q_user, q_post } = schema;

    // Alice (no inviter)
    drizzle_exec!(
        db.insert(q_user)
            .values([InsertQUser::new("Alice")])
            => execute
    );

    let all_users: Vec<SelectQUser> = drizzle_exec!(db.select(()).from(q_user) => all);
    let alice_id = all_users[0].id;

    // Bob (invited by Alice)
    drizzle_exec!(
        db.insert(q_user)
            .values([InsertQUser::new("Bob").with_invited_by(alice_id)])
            => execute
    );

    let all_users: Vec<SelectQUser> = drizzle_exec!(db.select(()).from(q_user) => all);
    let bob_id = all_users.iter().find(|u| u.name == "Bob").unwrap().id;

    // Posts by Bob
    drizzle_exec!(
        db.insert(q_post)
            .values([InsertQPost::new("Bob's Post", bob_id)])
            => execute
    );

    // Query users with both posts AND invited_by
    let users = drizzle_exec!(
        db.query(q_user)
            .with(q_user.q_posts())
            .with(q_user.invited_by())
            .find_many()
    );

    assert_eq!(users.len(), 2);

    // Bob has 1 post and was invited by Alice
    let bob = users.iter().find(|u| u.name == "Bob").unwrap();
    assert_eq!(bob.q_posts().len(), 1);
    assert_eq!(bob.q_posts()[0].content, "Bob's Post");
    assert!(bob.invited_by().is_some());
    assert_eq!(bob.invited_by().as_ref().unwrap().name, "Alice");

    // Alice has no posts and no inviter
    let alice = users.iter().find(|u| u.name == "Alice").unwrap();
    assert_eq!(alice.q_posts().len(), 0);
    assert!(alice.invited_by().is_none());
});

// -- Empty relation (Many with no rows) --
postgres_test!(query_empty_many_relation, QUserPostSchema, {
    let QUserPostSchema { q_user, q_post: _ } = schema;

    drizzle_exec!(
        db.insert(q_user)
            .values([InsertQUser::new("Alice")])
            => execute
    );

    let users = drizzle_exec!(db.query(q_user).with(q_user.q_posts()).find_many());

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].q_posts().len(), 0);
});

// -- Typed WHERE on root query (tests $N placeholder renumbering) --
postgres_test!(query_where_typed, QUserPostSchema, {
    let QUserPostSchema { q_user, q_post: _ } = schema;

    drizzle_exec!(
        db.insert(q_user)
            .values([
                InsertQUser::new("Alice"),
                InsertQUser::new("Bob"),
                InsertQUser::new("Charlie"),
            ])
            => execute
    );

    // Filter with typed expression
    let users = drizzle_exec!(db.query(q_user).r#where(eq(q_user.name, "Bob")).find_many());

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "Bob");
});

// -- Typed ORDER BY on root query --
postgres_test!(query_order_by_typed, QUserPostSchema, {
    let QUserPostSchema { q_user, q_post: _ } = schema;

    drizzle_exec!(
        db.insert(q_user)
            .values([
                InsertQUser::new("Charlie"),
                InsertQUser::new("Alice"),
                InsertQUser::new("Bob"),
            ])
            => execute
    );

    // Order by name ascending
    let users = drizzle_exec!(db.query(q_user).order_by(asc(q_user.name)).find_many());

    assert_eq!(users[0].name, "Alice");
    assert_eq!(users[1].name, "Bob");
    assert_eq!(users[2].name, "Charlie");

    // Order by name descending
    let users = drizzle_exec!(db.query(q_user).order_by(desc(q_user.name)).find_many());

    assert_eq!(users[0].name, "Charlie");
    assert_eq!(users[1].name, "Bob");
    assert_eq!(users[2].name, "Alice");
});

// -- Typed WHERE on relation subquery (tests $N renumbering in subqueries) --
postgres_test!(query_relation_where_typed, QUserPostSchema, {
    let QUserPostSchema { q_user, q_post } = schema;

    drizzle_exec!(
        db.insert(q_user)
            .values([InsertQUser::new("Alice")])
            => execute
    );

    let all_users: Vec<SelectQUser> = drizzle_exec!(db.select(()).from(q_user) => all);
    let alice_id = all_users[0].id;

    drizzle_exec!(
        db.insert(q_post)
            .values([
                InsertQPost::new("Post A", alice_id),
                InsertQPost::new("Post B", alice_id),
                InsertQPost::new("Post C", alice_id),
            ])
            => execute
    );

    let all_posts: Vec<SelectQPost> = drizzle_exec!(db.select(()).from(q_post) => all);
    let threshold_id = all_posts.iter().find(|p| p.content == "Post A").unwrap().id;

    // Only include posts with id > threshold (should exclude "Post A")
    let users = drizzle_exec!(
        db.query(q_user)
            .with(q_user.q_posts().r#where(gt(q_post.id, threshold_id)))
            .find_many()
    );

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].q_posts().len(), 2);
});

// -- Typed ORDER BY + LIMIT on relation subquery --
postgres_test!(query_relation_order_limit_typed, QUserPostSchema, {
    let QUserPostSchema { q_user, q_post } = schema;

    drizzle_exec!(
        db.insert(q_user)
            .values([InsertQUser::new("Alice")])
            => execute
    );

    let all_users: Vec<SelectQUser> = drizzle_exec!(db.select(()).from(q_user) => all);
    let alice_id = all_users[0].id;

    drizzle_exec!(
        db.insert(q_post)
            .values([
                InsertQPost::new("Post C", alice_id),
                InsertQPost::new("Post A", alice_id),
                InsertQPost::new("Post B", alice_id),
            ])
            => execute
    );

    // Order posts by content desc, take first 2
    let users = drizzle_exec!(
        db.query(q_user)
            .with(q_user.q_posts().order_by(desc(q_post.content)).limit(2),)
            .find_many()
    );

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].q_posts().len(), 2);
    assert_eq!(users[0].q_posts()[0].content, "Post C");
    assert_eq!(users[0].q_posts()[1].content, "Post B");
});

// =============================================================================
// View support
// =============================================================================

#[PostgresView(DEFINITION = "SELECT id, content, author_id FROM q_post")]
struct QPostView {
    id: i32,
    content: String,
    author_id: i32,
}

#[derive(PostgresSchema)]
struct QViewSchema {
    q_user: QUser,
    q_post: QPost,
    q_post_view: QPostView,
}

// -- Basic view query without relations --
postgres_test!(query_view_find_many, QViewSchema, {
    let QViewSchema {
        q_user,
        q_post,
        q_post_view,
    } = schema;

    drizzle_exec!(
        db.insert(q_user)
            .values([InsertQUser::new("Alice")])
            => execute
    );

    let all_users: Vec<SelectQUser> = drizzle_exec!(db.select(()).from(q_user) => all);
    let alice_id = all_users[0].id;

    drizzle_exec!(
        db.insert(q_post)
            .values([
                InsertQPost::new("Post 1", alice_id),
                InsertQPost::new("Post 2", alice_id),
            ])
            => execute
    );

    let posts = drizzle_exec!(db.query(q_post_view).find_many());
    assert_eq!(posts.len(), 2);
    assert_eq!(posts[0].content, "Post 1");
    assert_eq!(posts[1].content, "Post 2");
});

// -- View with find_first --
postgres_test!(query_view_find_first, QViewSchema, {
    let QViewSchema {
        q_user,
        q_post,
        q_post_view,
    } = schema;

    drizzle_exec!(
        db.insert(q_user)
            .values([InsertQUser::new("Alice")])
            => execute
    );

    let all_users: Vec<SelectQUser> = drizzle_exec!(db.select(()).from(q_user) => all);
    let alice_id = all_users[0].id;

    drizzle_exec!(
        db.insert(q_post)
            .values([
                InsertQPost::new("First Post", alice_id),
                InsertQPost::new("Second Post", alice_id),
            ])
            => execute
    );

    let post = drizzle_exec!(db.query(q_post_view).find_first());
    assert!(post.is_some());
    assert_eq!(post.unwrap().content, "First Post");
});

// -- View with WHERE and ORDER BY --
postgres_test!(query_view_where_order, QViewSchema, {
    let QViewSchema {
        q_user,
        q_post,
        q_post_view,
    } = schema;

    drizzle_exec!(
        db.insert(q_user)
            .values([InsertQUser::new("Alice")])
            => execute
    );

    let all_users: Vec<SelectQUser> = drizzle_exec!(db.select(()).from(q_user) => all);
    let alice_id = all_users[0].id;

    drizzle_exec!(
        db.insert(q_post)
            .values([
                InsertQPost::new("Charlie Post", alice_id),
                InsertQPost::new("Alpha Post", alice_id),
                InsertQPost::new("Bravo Post", alice_id),
            ])
            => execute
    );

    // Order by content ascending
    let posts = drizzle_exec!(
        db.query(q_post_view)
            .order_by(asc(q_post_view.content))
            .find_many()
    );

    assert_eq!(posts[0].content, "Alpha Post");
    assert_eq!(posts[1].content, "Bravo Post");
    assert_eq!(posts[2].content, "Charlie Post");

    // ORDER BY DESC + LIMIT
    let posts = drizzle_exec!(
        db.query(q_post_view)
            .order_by(desc(q_post_view.content))
            .limit(2)
            .find_many()
    );

    assert_eq!(posts.len(), 2);
    assert_eq!(posts[0].content, "Charlie Post");
    assert_eq!(posts[1].content, "Bravo Post");
});

// -- Complex deeply nested: 4-level deep, multiple siblings, all cardinalities --
postgres_test!(query_deep_nested_complex, QDeepSchema, {
    let QDeepSchema {
        q_user,
        q_post,
        q_comment,
        q_reply,
    } = schema;

    // === Seed data ===

    // Users: Alice (no inviter), then Bob/Charlie (invited by Alice), Dave (no inviter)
    drizzle_exec!(
        db.insert(q_user)
            .values([InsertQUser::new("Alice")])
            => execute
    );
    let all_users: Vec<SelectQUser> = drizzle_exec!(db.select(()).from(q_user) => all);
    let alice_id = all_users[0].id;

    drizzle_exec!(
        db.insert(q_user)
            .values([
                InsertQUser::new("Bob").with_invited_by(alice_id),
                InsertQUser::new("Charlie").with_invited_by(alice_id),
            ])
            => execute
    );
    drizzle_exec!(
        db.insert(q_user)
            .values([InsertQUser::new("Dave")])
            => execute
    );
    let all_users: Vec<SelectQUser> = drizzle_exec!(db.select(()).from(q_user) => all);
    let bob_id = all_users.iter().find(|u| u.name == "Bob").unwrap().id;

    // Posts: Alice has 4 posts, Bob has 1, Charlie/Dave have none
    drizzle_exec!(
        db.insert(q_post)
            .values([
                InsertQPost::new("Alice Draft", alice_id),
                InsertQPost::new("Alice Thoughts", alice_id),
                InsertQPost::new("Alice Update", alice_id),
                InsertQPost::new("Alice Announcement", alice_id),
                InsertQPost::new("Bob First Post", bob_id),
            ])
            => execute
    );
    let all_posts: Vec<SelectQPost> = drizzle_exec!(db.select(()).from(q_post) => all);
    let alice_draft_id = all_posts
        .iter()
        .find(|p| p.content == "Alice Draft")
        .unwrap()
        .id;
    let alice_thoughts_id = all_posts
        .iter()
        .find(|p| p.content == "Alice Thoughts")
        .unwrap()
        .id;
    let bob_post_id = all_posts
        .iter()
        .find(|p| p.content == "Bob First Post")
        .unwrap()
        .id;

    // Comments: 3 on Alice's Draft, 1 on Thoughts, 1 on Bob's post, 0 on others
    drizzle_exec!(
        db.insert(q_comment)
            .values([
                InsertQComment::new("Great draft!", alice_draft_id),
                InsertQComment::new("Needs work", alice_draft_id),
                InsertQComment::new("Love this", alice_draft_id),
                InsertQComment::new("Interesting thoughts", alice_thoughts_id),
                InsertQComment::new("Welcome Bob!", bob_post_id),
            ])
            => execute
    );
    let all_comments: Vec<SelectQComment> = drizzle_exec!(db.select(()).from(q_comment) => all);
    let great_draft_id = all_comments
        .iter()
        .find(|c| c.body == "Great draft!")
        .unwrap()
        .id;
    let needs_work_id = all_comments
        .iter()
        .find(|c| c.body == "Needs work")
        .unwrap()
        .id;
    let welcome_bob_id = all_comments
        .iter()
        .find(|c| c.body == "Welcome Bob!")
        .unwrap()
        .id;

    // Replies: on "Great draft!" (1), "Needs work" (1), "Welcome Bob!" (1), others (0)
    drizzle_exec!(
        db.insert(q_reply)
            .values([
                InsertQReply::new("Thanks!", great_draft_id),
                InsertQReply::new("Will revise", needs_work_id),
                InsertQReply::new("Glad to be here", welcome_bob_id),
            ])
            => execute
    );

    // === Complex query ===
    // 4-level deep: User -> Posts -> Comments -> Replies
    // Multiple sibling relations on root: posts + invited_by
    // ORDER BY + LIMIT on nested Many relation (triggers inner subquery)
    // ORDER BY on comments
    // Root ORDER BY
    let users = drizzle_exec!(
        db.query(q_user)
            .with(
                q_user
                    .q_posts()
                    .order_by(desc(q_post.content))
                    .limit(3)
                    .with(
                        q_post
                            .q_comments()
                            .order_by(asc(q_comment.body))
                            .with(q_comment.q_replys()),
                    ),
            )
            .with(q_user.invited_by())
            .order_by(asc(q_user.name))
            .find_many()
    );

    // === Assertions ===
    assert_eq!(users.len(), 4); // Alice, Bob, Charlie, Dave (ordered by name)
    assert_eq!(users[0].name, "Alice");
    assert_eq!(users[1].name, "Bob");
    assert_eq!(users[2].name, "Charlie");
    assert_eq!(users[3].name, "Dave");

    // -- Alice: no inviter, 4 posts but LIMIT 3 --
    assert!(users[0].invited_by().is_none());
    let alice_posts = users[0].q_posts();
    // LIMIT 3, ordered by content DESC: "Update", "Thoughts", "Draft" (Announcement excluded)
    assert_eq!(alice_posts.len(), 3);
    assert_eq!(alice_posts[0].content, "Alice Update");
    assert_eq!(alice_posts[1].content, "Alice Thoughts");
    assert_eq!(alice_posts[2].content, "Alice Draft");

    // Alice Update: 0 comments
    assert_eq!(alice_posts[0].q_comments().len(), 0);

    // Alice Thoughts: 1 comment, no replies
    assert_eq!(alice_posts[1].q_comments().len(), 1);
    assert_eq!(alice_posts[1].q_comments()[0].body, "Interesting thoughts");
    assert_eq!(alice_posts[1].q_comments()[0].q_replys().len(), 0);

    // Alice Draft: 3 comments ordered by body ASC
    let draft_comments = alice_posts[2].q_comments();
    assert_eq!(draft_comments.len(), 3);
    assert_eq!(draft_comments[0].body, "Great draft!");
    assert_eq!(draft_comments[1].body, "Love this");
    assert_eq!(draft_comments[2].body, "Needs work");
    // "Great draft!" has 1 reply
    assert_eq!(draft_comments[0].q_replys().len(), 1);
    assert_eq!(draft_comments[0].q_replys()[0].text, "Thanks!");
    // "Love this" has 0 replies
    assert_eq!(draft_comments[1].q_replys().len(), 0);
    // "Needs work" has 1 reply
    assert_eq!(draft_comments[2].q_replys().len(), 1);
    assert_eq!(draft_comments[2].q_replys()[0].text, "Will revise");

    // -- Bob: invited by Alice, 1 post with 1 comment with 1 reply --
    assert!(users[1].invited_by().is_some());
    assert_eq!(users[1].invited_by().as_ref().unwrap().name, "Alice");
    assert_eq!(users[1].q_posts().len(), 1);
    assert_eq!(users[1].q_posts()[0].content, "Bob First Post");
    assert_eq!(users[1].q_posts()[0].q_comments().len(), 1);
    assert_eq!(users[1].q_posts()[0].q_comments()[0].body, "Welcome Bob!");
    assert_eq!(users[1].q_posts()[0].q_comments()[0].q_replys().len(), 1);
    assert_eq!(
        users[1].q_posts()[0].q_comments()[0].q_replys()[0].text,
        "Glad to be here"
    );

    // -- Charlie: invited by Alice, no posts --
    assert!(users[2].invited_by().is_some());
    assert_eq!(users[2].invited_by().as_ref().unwrap().name, "Alice");
    assert_eq!(users[2].q_posts().len(), 0);

    // -- Dave: no inviter, no posts --
    assert!(users[3].invited_by().is_none());
    assert_eq!(users[3].q_posts().len(), 0);
});

// =============================================================================
// Partial Column Selection
// =============================================================================

// -- Whitelist: select only specific columns --
postgres_test!(query_columns_whitelist, QUserPostSchema, {
    let QUserPostSchema { q_user, q_post: _ } = schema;

    drizzle_exec!(
        db.insert(q_user)
            .values([
                InsertQUser::new("Alice"),
                InsertQUser::new("Bob"),
            ])
            => execute
    );

    // Select only id and name (omitting invited_by)
    let users = drizzle_exec!(
        db.query(q_user)
            .columns(q_user.select_columns().id().name())
            .find_many()
    );

    assert_eq!(users.len(), 2);
    // Selected columns are Some
    assert!(users[0].id.is_some());
    assert!(users[0].name.is_some());
    assert_eq!(users[0].name.as_deref(), Some("Alice"));
    // Unselected columns are None
    assert!(users[0].invited_by.is_none());

    assert_eq!(users[1].name.as_deref(), Some("Bob"));
});

// -- Blacklist: omit specific columns --
postgres_test!(query_omit_blacklist, QUserPostSchema, {
    let QUserPostSchema { q_user, q_post: _ } = schema;

    drizzle_exec!(
        db.insert(q_user)
            .values([InsertQUser::new("Alice")])
            => execute
    );

    // Omit invited_by — should still return id and name
    let users = drizzle_exec!(
        db.query(q_user)
            .omit(q_user.select_columns().invited_by())
            .find_many()
    );

    assert_eq!(users.len(), 1);
    assert!(users[0].id.is_some());
    assert_eq!(users[0].name.as_deref(), Some("Alice"));
    // Omitted column is None
    assert!(users[0].invited_by.is_none());
});

// -- Partial columns with relations --
postgres_test!(query_columns_with_relations, QUserPostSchema, {
    let QUserPostSchema { q_user, q_post } = schema;

    drizzle_exec!(
        db.insert(q_user)
            .values([InsertQUser::new("Alice")])
            => execute
    );

    let all_users: Vec<SelectQUser> = drizzle_exec!(db.select(()).from(q_user) => all);
    let alice_id = all_users[0].id;

    drizzle_exec!(
        db.insert(q_post)
            .values([
                InsertQPost::new("Post 1", alice_id),
                InsertQPost::new("Post 2", alice_id),
            ])
            => execute
    );

    // Partial columns on base, full relations
    let users = drizzle_exec!(
        db.query(q_user)
            .columns(q_user.select_columns().id().name())
            .with(q_user.q_posts())
            .find_many()
    );

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name.as_deref(), Some("Alice"));
    assert!(users[0].invited_by.is_none()); // not selected
    assert_eq!(users[0].q_posts().len(), 2);
    // Relations are full SelectModel (not partial)
    assert_eq!(users[0].q_posts()[0].content, "Post 1");
});

// -- Partial columns on a relation --
postgres_test!(query_relation_columns, QUserPostSchema, {
    let QUserPostSchema { q_user, q_post } = schema;

    drizzle_exec!(
        db.insert(q_user)
            .values([InsertQUser::new("Alice")])
            => execute
    );

    let all_users: Vec<SelectQUser> = drizzle_exec!(db.select(()).from(q_user) => all);
    let alice_id = all_users[0].id;

    drizzle_exec!(
        db.insert(q_post)
            .values([
                InsertQPost::new("Post 1", alice_id),
                InsertQPost::new("Post 2", alice_id),
            ])
            => execute
    );

    // Full base, partial columns on relation
    let users = drizzle_exec!(
        db.query(q_user)
            .with(
                q_user
                    .q_posts()
                    .columns(q_post.select_columns().id().content())
            )
            .find_many()
    );

    assert_eq!(users.len(), 1);
    // Base is full SelectModel
    assert_eq!(users[0].name, "Alice");
    // Relation is PartialSelectModel
    assert_eq!(users[0].q_posts().len(), 2);
    assert!(users[0].q_posts()[0].id.is_some());
    assert_eq!(users[0].q_posts()[0].content.as_deref(), Some("Post 1"));
    // author_id not selected
    assert!(users[0].q_posts()[0].author_id.is_none());
});

// -- find_first with partial columns --
postgres_test!(query_columns_find_first, QUserPostSchema, {
    let QUserPostSchema { q_user, q_post: _ } = schema;

    drizzle_exec!(
        db.insert(q_user)
            .values([InsertQUser::new("Alice")])
            => execute
    );

    let user = drizzle_exec!(
        db.query(q_user)
            .columns(q_user.select_columns().name())
            .find_first()
    );

    assert!(user.is_some());
    let user = user.unwrap();
    assert_eq!(user.name.as_deref(), Some("Alice"));
    assert!(user.id.is_none()); // not selected
});
