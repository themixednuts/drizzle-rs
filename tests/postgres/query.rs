#![cfg(all(
    any(feature = "postgres-sync", feature = "tokio-postgres"),
    feature = "query",
    feature = "uuid"
))]

use crate::common::schema::postgres::{
    Comment, Complex, InsertComment, InsertComplex, InsertPost, InsertReply, Post, Reply, Role,
    SelectComment, SelectComplex, SelectPost,
};
use drizzle::core::expr::{eq, gt};
use drizzle::core::{asc, desc};
use drizzle::postgres::prelude::*;
use drizzle_macros::postgres_test;
use uuid::Uuid;

// Import generated relation accessor traits from the common schema.
// These are needed because the table definitions live in a different module.
#[allow(unused_imports)]
use crate::common::schema::postgres::{
    __ColumnsAccessor_Complex, __ColumnsAccessor_Post, __Comment_Replys_RelAccessor,
    __Complex_InvitedBy_RelAccessor, __Complex_Posts_RelAccessor, __Post_Author_RelAccessor,
    __Post_Comments_RelAccessor, __QueryAccess_Comment_Replys, __QueryAccess_Complex_InvitedBy,
    __QueryAccess_Complex_Posts, __QueryAccess_Post_Author, __QueryAccess_Post_Comments, ComplexId,
    ComplexWithInvitedBy, ComplexWithPosts,
};

// =============================================================================
// Schemas for different test scenarios
// =============================================================================

#[derive(PostgresSchema)]
struct ComplexPostQuerySchema {
    role: Role,
    complex: Complex,
    post: Post,
}

#[derive(PostgresSchema)]
struct FullQuerySchema {
    role: Role,
    complex: Complex,
    post: Post,
    comment: Comment,
}

#[derive(PostgresSchema)]
struct DeepQuerySchema {
    role: Role,
    complex: Complex,
    post: Post,
    comment: Comment,
    reply: Reply,
}

// =============================================================================
// Tests
// =============================================================================

// -- Basic find_many without relations --
postgres_test!(query_find_many_no_relations, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema {
        complex, post: _, ..
    } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([
                InsertComplex::new("Alice", true, Role::User),
                InsertComplex::new("Bob", true, Role::User),
            ])
            => execute
    );

    let users = drizzle_exec!(db.query(complex).order_by(asc(complex.name)).find_many());
    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "Alice");
    assert_eq!(users[1].name, "Bob");
});

// -- find_first --
postgres_test!(query_find_first, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema {
        complex, post: _, ..
    } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([
                InsertComplex::new("Alice", true, Role::User),
                InsertComplex::new("Bob", true, Role::User),
            ])
            => execute
    );

    let user = drizzle_exec!(db.query(complex).order_by(asc(complex.name)).find_first());
    assert!(user.is_some());
    assert_eq!(user.unwrap().name, "Alice");
});

// -- find_first returns None on empty table --
postgres_test!(query_find_first_empty, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema {
        complex, post: _, ..
    } = schema;

    let user = drizzle_exec!(db.query(complex).find_first());
    assert!(user.is_none());
});

// -- with limit --
postgres_test!(query_with_limit, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema {
        complex, post: _, ..
    } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([
                InsertComplex::new("Alice", true, Role::User),
                InsertComplex::new("Bob", true, Role::User),
                InsertComplex::new("Charlie", true, Role::User),
            ])
            => execute
    );

    let users = drizzle_exec!(db.query(complex).limit(2).find_many());
    assert_eq!(users.len(), 2);
});

// -- Reverse relation: Complex -> Posts (Many) --
postgres_test!(query_reverse_relation_many, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post, .. } = schema;

    // Insert users
    drizzle_exec!(
        db.insert(complex)
            .values([
                InsertComplex::new("Alice", true, Role::User),
                InsertComplex::new("Bob", true, Role::User),
            ])
            => execute
    );

    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let alice_id = all_users.iter().find(|u| u.name == "Alice").unwrap().id;
    let bob_id = all_users.iter().find(|u| u.name == "Bob").unwrap().id;

    // Insert posts
    drizzle_exec!(
        db.insert(post)
            .values([
                InsertPost::new("Alice Post 1", true).with_author_id(alice_id),
                InsertPost::new("Alice Post 2", true).with_author_id(alice_id),
                InsertPost::new("Bob Post 1", true).with_author_id(bob_id),
            ])
            => execute
    );

    // Query users with their posts
    let users = drizzle_exec!(db.query(complex).with(complex.posts()).find_many());

    assert_eq!(users.len(), 2);

    // Alice has 2 posts
    let alice = users.iter().find(|u| u.name == "Alice").unwrap();
    assert_eq!(alice.posts().len(), 2);
    assert_eq!(alice.posts()[0].title, "Alice Post 1");
    assert_eq!(alice.posts()[1].title, "Alice Post 2");

    // Bob has 1 post
    let bob = users.iter().find(|u| u.name == "Bob").unwrap();
    assert_eq!(bob.posts().len(), 1);
    assert_eq!(bob.posts()[0].title, "Bob Post 1");
});

// -- Forward relation: Post -> Author (OptionalOne since author_id is nullable) --
postgres_test!(query_forward_relation_one, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post, .. } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );

    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let alice_id = all_users[0].id;

    drizzle_exec!(
        db.insert(post)
            .values([InsertPost::new("Hello World", true).with_author_id(alice_id)])
            => execute
    );

    // Query posts with their author
    let posts = drizzle_exec!(db.query(post).with(post.author()).find_many());

    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0].title, "Hello World");
    assert_eq!(posts[0].author().as_ref().unwrap().name, "Alice");
});

// -- Forward relation: OptionalOne (nullable FK, self-referential) --
postgres_test!(query_forward_optional_one, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema {
        complex, post: _, ..
    } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );

    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let alice_id = all_users[0].id;

    // Bob was invited by Alice
    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Bob", true, Role::User).with_invited_by(alice_id)])
            => execute
    );

    // Query users with their inviter
    let users = drizzle_exec!(db.query(complex).with(complex.invited_by()).find_many());

    assert_eq!(users.len(), 2);

    // Alice has no inviter
    let alice = users.iter().find(|u| u.name == "Alice").unwrap();
    assert!(alice.invited_by().is_none());

    // Bob was invited by Alice
    let bob = users.iter().find(|u| u.name == "Bob").unwrap();
    assert!(bob.invited_by().is_some());
    assert_eq!(bob.invited_by().as_ref().unwrap().name, "Alice");
});

// -- Nested relations: Complex -> Posts -> Comments --
postgres_test!(query_nested_relations, FullQuerySchema, {
    let FullQuerySchema {
        complex,
        post,
        comment,
        ..
    } = schema;

    // Insert user
    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );

    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let alice_id = all_users[0].id;

    // Insert posts
    drizzle_exec!(
        db.insert(post)
            .values([
                InsertPost::new("Post 1", true).with_author_id(alice_id),
                InsertPost::new("Post 2", true).with_author_id(alice_id),
            ])
            => execute
    );

    let all_posts: Vec<SelectPost> = drizzle_exec!(db.select(()).from(post) => all);
    let post1_id = all_posts.iter().find(|p| p.title == "Post 1").unwrap().id;
    let post2_id = all_posts.iter().find(|p| p.title == "Post 2").unwrap().id;

    // Insert comments
    drizzle_exec!(
        db.insert(comment)
            .values([
                InsertComment::new("Comment on P1-A", post1_id),
                InsertComment::new("Comment on P1-B", post1_id),
                InsertComment::new("Comment on P2", post2_id),
            ])
            => execute
    );

    // Query: users -> posts -> comments
    let users = drizzle_exec!(
        db.query(complex)
            .with(complex.posts().with(post.comments()))
            .find_many()
    );

    assert_eq!(users.len(), 1);
    let alice = &users[0];
    assert_eq!(alice.name, "Alice");
    assert_eq!(alice.posts().len(), 2);

    // Find post 1 and check its comments
    let p1 = alice.posts().iter().find(|p| p.title == "Post 1").unwrap();
    assert_eq!(p1.comments().len(), 2);

    let p2 = alice.posts().iter().find(|p| p.title == "Post 2").unwrap();
    assert_eq!(p2.comments().len(), 1);
    assert_eq!(p2.comments()[0].body, "Comment on P2");
});

// -- Multiple relations: Complex with posts AND invited_by --
postgres_test!(query_multiple_relations, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post, .. } = schema;

    // Alice (no inviter)
    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );

    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let alice_id = all_users[0].id;

    // Bob (invited by Alice)
    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Bob", true, Role::User).with_invited_by(alice_id)])
            => execute
    );

    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let bob_id = all_users.iter().find(|u| u.name == "Bob").unwrap().id;

    // Posts by Bob
    drizzle_exec!(
        db.insert(post)
            .values([InsertPost::new("Bob's Post", true).with_author_id(bob_id)])
            => execute
    );

    // Query users with both posts AND invited_by
    let users = drizzle_exec!(
        db.query(complex)
            .with(complex.posts())
            .with(complex.invited_by())
            .find_many()
    );

    assert_eq!(users.len(), 2);

    // Bob has 1 post and was invited by Alice
    let bob = users.iter().find(|u| u.name == "Bob").unwrap();
    assert_eq!(bob.posts().len(), 1);
    assert_eq!(bob.posts()[0].title, "Bob's Post");
    assert!(bob.invited_by().is_some());
    assert_eq!(bob.invited_by().as_ref().unwrap().name, "Alice");

    // Alice has no posts and no inviter
    let alice = users.iter().find(|u| u.name == "Alice").unwrap();
    assert_eq!(alice.posts().len(), 0);
    assert!(alice.invited_by().is_none());
});

// -- Empty relation (Many with no rows) --
postgres_test!(query_empty_many_relation, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema {
        complex, post: _, ..
    } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );

    let users = drizzle_exec!(db.query(complex).with(complex.posts()).find_many());

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].posts().len(), 0);
});

// -- Typed WHERE on root query (tests $N placeholder renumbering) --
postgres_test!(query_where_typed, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema {
        complex, post: _, ..
    } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([
                InsertComplex::new("Alice", true, Role::User),
                InsertComplex::new("Bob", true, Role::User),
                InsertComplex::new("Charlie", true, Role::User),
            ])
            => execute
    );

    // Filter with typed expression
    let users = drizzle_exec!(
        db.query(complex)
            .r#where(eq(complex.name, "Bob"))
            .find_many()
    );

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "Bob");
});

// -- Typed ORDER BY on root query --
postgres_test!(query_order_by_typed, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema {
        complex, post: _, ..
    } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([
                InsertComplex::new("Charlie", true, Role::User),
                InsertComplex::new("Alice", true, Role::User),
                InsertComplex::new("Bob", true, Role::User),
            ])
            => execute
    );

    // Order by name ascending
    let users = drizzle_exec!(db.query(complex).order_by(asc(complex.name)).find_many());

    assert_eq!(users[0].name, "Alice");
    assert_eq!(users[1].name, "Bob");
    assert_eq!(users[2].name, "Charlie");

    // Order by name descending
    let users = drizzle_exec!(db.query(complex).order_by(desc(complex.name)).find_many());

    assert_eq!(users[0].name, "Charlie");
    assert_eq!(users[1].name, "Bob");
    assert_eq!(users[2].name, "Alice");
});

// -- Typed WHERE on relation subquery (tests $N renumbering in subqueries) --
postgres_test!(query_relation_where_typed, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post, .. } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );

    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let alice_id = all_users[0].id;

    drizzle_exec!(
        db.insert(post)
            .values([
                InsertPost::new("Post A", true).with_author_id(alice_id),
                InsertPost::new("Post B", true).with_author_id(alice_id),
                InsertPost::new("Post C", true).with_author_id(alice_id),
            ])
            => execute
    );

    let all_posts: Vec<SelectPost> = drizzle_exec!(db.select(()).from(post) => all);
    let threshold_id = all_posts.iter().find(|p| p.title == "Post A").unwrap().id;

    // Only include posts with id > threshold (should exclude "Post A")
    let users = drizzle_exec!(
        db.query(complex)
            .with(complex.posts().r#where(gt(post.id, threshold_id)))
            .find_many()
    );

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].posts().len(), 2);
});

// -- Typed ORDER BY + LIMIT on relation subquery --
postgres_test!(query_relation_order_limit_typed, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post, .. } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );

    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let alice_id = all_users[0].id;

    drizzle_exec!(
        db.insert(post)
            .values([
                InsertPost::new("Post C", true).with_author_id(alice_id),
                InsertPost::new("Post A", true).with_author_id(alice_id),
                InsertPost::new("Post B", true).with_author_id(alice_id),
            ])
            => execute
    );

    // Order posts by title desc, take first 2
    let users = drizzle_exec!(
        db.query(complex)
            .with(complex.posts().order_by(desc(post.title)).limit(2),)
            .find_many()
    );

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].posts().len(), 2);
    assert_eq!(users[0].posts()[0].title, "Post C");
    assert_eq!(users[0].posts()[1].title, "Post B");
});

// =============================================================================
// View support
// =============================================================================

#[PostgresView(DEFINITION = "SELECT id, title, author_id FROM post")]
struct PostView {
    id: Uuid,
    title: String,
    author_id: Option<Uuid>,
}

#[derive(PostgresSchema)]
struct ViewQuerySchema {
    role: Role,
    complex: Complex,
    post: Post,
    post_view: PostView,
}

// -- Basic view query without relations --
postgres_test!(query_view_find_many, ViewQuerySchema, {
    let ViewQuerySchema {
        complex,
        post,
        post_view,
        ..
    } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );

    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let alice_id = all_users[0].id;

    drizzle_exec!(
        db.insert(post)
            .values([
                InsertPost::new("Post 1", true).with_author_id(alice_id),
                InsertPost::new("Post 2", true).with_author_id(alice_id),
            ])
            => execute
    );

    let posts = drizzle_exec!(db.query(post_view).find_many());
    assert_eq!(posts.len(), 2);
    assert_eq!(posts[0].title, "Post 1");
    assert_eq!(posts[1].title, "Post 2");
});

// -- View with find_first --
postgres_test!(query_view_find_first, ViewQuerySchema, {
    let ViewQuerySchema {
        complex,
        post,
        post_view,
        ..
    } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );

    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let alice_id = all_users[0].id;

    drizzle_exec!(
        db.insert(post)
            .values([
                InsertPost::new("First Post", true).with_author_id(alice_id),
                InsertPost::new("Second Post", true).with_author_id(alice_id),
            ])
            => execute
    );

    let found = drizzle_exec!(db.query(post_view).find_first());
    assert!(found.is_some());
    assert_eq!(found.unwrap().title, "First Post");
});

// -- View with WHERE and ORDER BY --
postgres_test!(query_view_where_order, ViewQuerySchema, {
    let ViewQuerySchema {
        complex,
        post,
        post_view,
        ..
    } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );

    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let alice_id = all_users[0].id;

    drizzle_exec!(
        db.insert(post)
            .values([
                InsertPost::new("Charlie Post", true).with_author_id(alice_id),
                InsertPost::new("Alpha Post", true).with_author_id(alice_id),
                InsertPost::new("Bravo Post", true).with_author_id(alice_id),
            ])
            => execute
    );

    // Order by title ascending
    let posts = drizzle_exec!(
        db.query(post_view)
            .order_by(asc(post_view.title))
            .find_many()
    );

    assert_eq!(posts[0].title, "Alpha Post");
    assert_eq!(posts[1].title, "Bravo Post");
    assert_eq!(posts[2].title, "Charlie Post");

    // ORDER BY DESC + LIMIT
    let posts = drizzle_exec!(
        db.query(post_view)
            .order_by(desc(post_view.title))
            .limit(2)
            .find_many()
    );

    assert_eq!(posts.len(), 2);
    assert_eq!(posts[0].title, "Charlie Post");
    assert_eq!(posts[1].title, "Bravo Post");
});

// -- View with FK: query a view that has relations --
#[PostgresView(DEFINITION = "SELECT id, title, author_id FROM post")]
struct PostViewFk {
    id: Uuid,
    title: String,
    #[column(references = Complex::id)]
    author_id: Option<Uuid>,
}

#[derive(PostgresSchema)]
struct ViewFkQuerySchema {
    role: Role,
    complex: Complex,
    post: Post,
    post_view_fk: PostViewFk,
}

// -- View with forward relation (view -> table) --
postgres_test!(query_view_with_forward_relation, ViewFkQuerySchema, {
    let ViewFkQuerySchema {
        complex,
        post,
        post_view_fk,
        ..
    } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([
                InsertComplex::new("Alice", true, Role::User),
                InsertComplex::new("Bob", true, Role::User),
            ])
            => execute
    );

    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let alice_id = all_users.iter().find(|u| u.name == "Alice").unwrap().id;
    let bob_id = all_users.iter().find(|u| u.name == "Bob").unwrap().id;

    drizzle_exec!(
        db.insert(post)
            .values([
                InsertPost::new("Alice's Post", true).with_author_id(alice_id),
                InsertPost::new("Bob's Post", true).with_author_id(bob_id),
            ])
            => execute
    );

    // Query the view with its forward relation (author)
    let posts = drizzle_exec!(
        db.query(post_view_fk)
            .with(post_view_fk.author())
            .order_by(asc(post_view_fk.title))
            .find_many()
    );

    assert_eq!(posts.len(), 2);
    assert_eq!(posts[0].title, "Alice's Post");
    assert_eq!(posts[0].author().as_ref().unwrap().name, "Alice");
    assert_eq!(posts[1].title, "Bob's Post");
    assert_eq!(posts[1].author().as_ref().unwrap().name, "Bob");
});

// -- Combo: query regular tables and views in the same schema --
postgres_test!(query_combo_tables_and_views, ViewFkQuerySchema, {
    let ViewFkQuerySchema {
        complex,
        post,
        post_view_fk,
        ..
    } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([
                InsertComplex::new("Alice", true, Role::User),
                InsertComplex::new("Bob", true, Role::User),
            ])
            => execute
    );

    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let alice_id = all_users.iter().find(|u| u.name == "Alice").unwrap().id;
    let bob_id = all_users.iter().find(|u| u.name == "Bob").unwrap().id;

    drizzle_exec!(
        db.insert(post)
            .values([
                InsertPost::new("Post A", true).with_author_id(alice_id),
                InsertPost::new("Post B", true).with_author_id(alice_id),
                InsertPost::new("Post C", true).with_author_id(bob_id),
            ])
            => execute
    );

    // 1) Query regular table with relations
    let users = drizzle_exec!(
        db.query(complex)
            .with(complex.posts())
            .order_by(asc(complex.name))
            .find_many()
    );
    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "Alice");
    assert_eq!(users[0].posts().len(), 2);
    assert_eq!(users[1].name, "Bob");
    assert_eq!(users[1].posts().len(), 1);

    // 2) Query view with relations from the same schema
    let view_posts = drizzle_exec!(
        db.query(post_view_fk)
            .with(post_view_fk.author())
            .order_by(asc(post_view_fk.title))
            .find_many()
    );
    assert_eq!(view_posts.len(), 3);
    assert_eq!(view_posts[0].title, "Post A");
    assert_eq!(view_posts[0].author().as_ref().unwrap().name, "Alice");
    assert_eq!(view_posts[2].title, "Post C");
    assert_eq!(view_posts[2].author().as_ref().unwrap().name, "Bob");

    // 3) Query view standalone (no relations)
    let view_first = drizzle_exec!(
        db.query(post_view_fk)
            .r#where(eq(post_view_fk.title, "Post B"))
            .find_first()
    );
    assert!(view_first.is_some());
    assert_eq!(view_first.unwrap().title, "Post B");
});

// -- Complex deeply nested: 4-level deep, multiple siblings, all cardinalities --
postgres_test!(query_deep_nested_complex, DeepQuerySchema, {
    let DeepQuerySchema {
        complex,
        post,
        comment,
        reply,
        ..
    } = schema;

    // === Seed data ===

    // Users: Alice (no inviter), then Bob/Charlie (invited by Alice), Dave (no inviter)
    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );
    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let alice_id = all_users[0].id;

    drizzle_exec!(
        db.insert(complex)
            .values([
                InsertComplex::new("Bob", true, Role::User).with_invited_by(alice_id),
                InsertComplex::new("Charlie", true, Role::User).with_invited_by(alice_id),
            ])
            => execute
    );
    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Dave", true, Role::User)])
            => execute
    );
    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let bob_id = all_users.iter().find(|u| u.name == "Bob").unwrap().id;

    // Posts: Alice has 4 posts, Bob has 1, Charlie/Dave have none
    drizzle_exec!(
        db.insert(post)
            .values([
                InsertPost::new("Alice Draft", true).with_author_id(alice_id),
                InsertPost::new("Alice Thoughts", true).with_author_id(alice_id),
                InsertPost::new("Alice Update", true).with_author_id(alice_id),
                InsertPost::new("Alice Announcement", true).with_author_id(alice_id),
                InsertPost::new("Bob First Post", true).with_author_id(bob_id),
            ])
            => execute
    );
    let all_posts: Vec<SelectPost> = drizzle_exec!(db.select(()).from(post) => all);
    let alice_draft_id = all_posts
        .iter()
        .find(|p| p.title == "Alice Draft")
        .unwrap()
        .id;
    let alice_thoughts_id = all_posts
        .iter()
        .find(|p| p.title == "Alice Thoughts")
        .unwrap()
        .id;
    let bob_post_id = all_posts
        .iter()
        .find(|p| p.title == "Bob First Post")
        .unwrap()
        .id;

    // Comments: 3 on Alice's Draft, 1 on Thoughts, 1 on Bob's post, 0 on others
    drizzle_exec!(
        db.insert(comment)
            .values([
                InsertComment::new("Great draft!", alice_draft_id),
                InsertComment::new("Needs work", alice_draft_id),
                InsertComment::new("Love this", alice_draft_id),
                InsertComment::new("Interesting thoughts", alice_thoughts_id),
                InsertComment::new("Welcome Bob!", bob_post_id),
            ])
            => execute
    );
    let all_comments: Vec<SelectComment> = drizzle_exec!(db.select(()).from(comment) => all);
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
        db.insert(reply)
            .values([
                InsertReply::new("Thanks!", great_draft_id),
                InsertReply::new("Will revise", needs_work_id),
                InsertReply::new("Glad to be here", welcome_bob_id),
            ])
            => execute
    );

    // === Complex query ===
    // 4-level deep: Complex -> Posts -> Comments -> Replies
    // Multiple sibling relations on root: posts + invited_by
    // ORDER BY + LIMIT on nested Many relation (triggers inner subquery)
    // ORDER BY on comments
    // Root ORDER BY
    let users = drizzle_exec!(
        db.query(complex)
            .with(
                complex.posts().order_by(desc(post.title)).limit(3).with(
                    post.comments()
                        .order_by(asc(comment.body))
                        .with(comment.replys()),
                ),
            )
            .with(complex.invited_by())
            .order_by(asc(complex.name))
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
    let alice_posts = users[0].posts();
    // LIMIT 3, ordered by title DESC: "Update", "Thoughts", "Draft" (Announcement excluded)
    assert_eq!(alice_posts.len(), 3);
    assert_eq!(alice_posts[0].title, "Alice Update");
    assert_eq!(alice_posts[1].title, "Alice Thoughts");
    assert_eq!(alice_posts[2].title, "Alice Draft");

    // Alice Update: 0 comments
    assert_eq!(alice_posts[0].comments().len(), 0);

    // Alice Thoughts: 1 comment, no replies
    assert_eq!(alice_posts[1].comments().len(), 1);
    assert_eq!(alice_posts[1].comments()[0].body, "Interesting thoughts");
    assert_eq!(alice_posts[1].comments()[0].replys().len(), 0);

    // Alice Draft: 3 comments ordered by body ASC
    let draft_comments = alice_posts[2].comments();
    assert_eq!(draft_comments.len(), 3);
    assert_eq!(draft_comments[0].body, "Great draft!");
    assert_eq!(draft_comments[1].body, "Love this");
    assert_eq!(draft_comments[2].body, "Needs work");
    // "Great draft!" has 1 reply
    assert_eq!(draft_comments[0].replys().len(), 1);
    assert_eq!(draft_comments[0].replys()[0].text, "Thanks!");
    // "Love this" has 0 replies
    assert_eq!(draft_comments[1].replys().len(), 0);
    // "Needs work" has 1 reply
    assert_eq!(draft_comments[2].replys().len(), 1);
    assert_eq!(draft_comments[2].replys()[0].text, "Will revise");

    // -- Bob: invited by Alice, 1 post with 1 comment with 1 reply --
    assert!(users[1].invited_by().is_some());
    assert_eq!(users[1].invited_by().as_ref().unwrap().name, "Alice");
    assert_eq!(users[1].posts().len(), 1);
    assert_eq!(users[1].posts()[0].title, "Bob First Post");
    assert_eq!(users[1].posts()[0].comments().len(), 1);
    assert_eq!(users[1].posts()[0].comments()[0].body, "Welcome Bob!");
    assert_eq!(users[1].posts()[0].comments()[0].replys().len(), 1);
    assert_eq!(
        users[1].posts()[0].comments()[0].replys()[0].text,
        "Glad to be here"
    );

    // -- Charlie: invited by Alice, no posts --
    assert!(users[2].invited_by().is_some());
    assert_eq!(users[2].invited_by().as_ref().unwrap().name, "Alice");
    assert_eq!(users[2].posts().len(), 0);

    // -- Dave: no inviter, no posts --
    assert!(users[3].invited_by().is_none());
    assert_eq!(users[3].posts().len(), 0);
});

// =============================================================================
// Type alias ergonomics
// =============================================================================

// Verify generated type aliases work in function signatures.
// The query API generates aliases like `ComplexWithPosts<Rest = ()>` so users
// can write clean function signatures instead of spelling out RelEntry<__Rel_...>.
use drizzle::core::query::QueryRow;

// Single relation: Complex with posts loaded
type ComplexWithPostsRow = QueryRow<SelectComplex, ComplexWithPosts>;

// Composed relations: Complex with invited_by AND posts loaded.
// The Rest parameter chains them: `ComplexWithInvitedBy<ComplexWithPosts>`
// means "store has invited_by first, then posts".
// Note: order must match the .with() call order (last .with() is outermost).
type ComplexWithPostsAndInviter = QueryRow<SelectComplex, ComplexWithInvitedBy<ComplexWithPosts>>;

fn count_posts(user: &ComplexWithPostsRow) -> usize {
    user.posts().len()
}

fn get_inviter_name(user: &ComplexWithPostsAndInviter) -> Option<&str> {
    user.invited_by().as_ref().map(|u| u.name.as_str())
}

postgres_test!(query_type_alias_in_fn_signature, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post, .. } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );

    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let alice_id = all_users[0].id;

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Bob", true, Role::User).with_invited_by(alice_id)])
            => execute
    );

    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let bob_id = all_users.iter().find(|u| u.name == "Bob").unwrap().id;

    drizzle_exec!(
        db.insert(post)
            .values([
                InsertPost::new("Post 1", true).with_author_id(alice_id),
                InsertPost::new("Post 2", true).with_author_id(alice_id),
                InsertPost::new("Bob Post", true).with_author_id(bob_id),
            ])
            => execute
    );

    // Use type alias with single relation
    let users: Vec<ComplexWithPostsRow> =
        drizzle_exec!(db.query(complex).with(complex.posts()).find_many());

    let alice = users.iter().find(|u| u.name == "Alice").unwrap();
    assert_eq!(count_posts(alice), 2);

    let bob = users.iter().find(|u| u.name == "Bob").unwrap();
    assert_eq!(count_posts(bob), 1);

    // Use type alias with composed relations
    // .with() order: posts first, then invited_by
    // Type order: InvitedBy<Posts> (last .with() is outermost in the store)
    let users: Vec<ComplexWithPostsAndInviter> = drizzle_exec!(
        db.query(complex)
            .with(complex.posts())
            .with(complex.invited_by())
            .find_many()
    );

    let bob = users.iter().find(|u| u.name == "Bob").unwrap();
    assert_eq!(get_inviter_name(bob), Some("Alice"));

    let alice = users.iter().find(|u| u.name == "Alice").unwrap();
    assert_eq!(get_inviter_name(alice), None);
});

// =============================================================================
// Offset
// =============================================================================

// -- Root query offset --
postgres_test!(query_with_limit_offset, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema {
        complex, post: _, ..
    } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([
                InsertComplex::new("Alice", true, Role::User),
                InsertComplex::new("Bob", true, Role::User),
                InsertComplex::new("Charlie", true, Role::User),
                InsertComplex::new("Dave", true, Role::User),
            ])
            => execute
    );

    // LIMIT 2 OFFSET 1 with ORDER BY to ensure determinism
    let users = drizzle_exec!(
        db.query(complex)
            .order_by(asc(complex.name))
            .limit(2)
            .offset(1)
            .find_many()
    );

    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "Bob");
    assert_eq!(users[1].name, "Charlie");
});

// -- Relation handle offset --
postgres_test!(query_relation_limit_offset, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post, .. } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );

    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let alice_id = all_users[0].id;

    drizzle_exec!(
        db.insert(post)
            .values([
                InsertPost::new("AAA", true).with_author_id(alice_id),
                InsertPost::new("BBB", true).with_author_id(alice_id),
                InsertPost::new("CCC", true).with_author_id(alice_id),
                InsertPost::new("DDD", true).with_author_id(alice_id),
            ])
            => execute
    );

    // Relation subquery with ORDER BY + LIMIT + OFFSET
    let users = drizzle_exec!(
        db.query(complex)
            .with(complex.posts().order_by(asc(post.title)).limit(2).offset(1))
            .find_many()
    );

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].posts().len(), 2);
    assert_eq!(users[0].posts()[0].title, "BBB");
    assert_eq!(users[0].posts()[1].title, "CCC");
});

// =============================================================================
// Partial Column Selection
// =============================================================================

// -- Whitelist: select only specific columns --
postgres_test!(query_columns_whitelist, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema {
        complex, post: _, ..
    } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([
                InsertComplex::new("Alice", true, Role::User),
                InsertComplex::new("Bob", true, Role::User),
            ])
            => execute
    );

    // Select only id and name (omitting invited_by and others)
    let users = drizzle_exec!(
        db.query(complex)
            .columns(complex.select_columns().id().name())
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
postgres_test!(query_omit_blacklist, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema {
        complex, post: _, ..
    } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );

    // Omit invited_by — should still return id, name, etc.
    let users = drizzle_exec!(
        db.query(complex)
            .omit(complex.select_columns().invited_by())
            .find_many()
    );

    assert_eq!(users.len(), 1);
    assert!(users[0].id.is_some());
    assert_eq!(users[0].name.as_deref(), Some("Alice"));
    // Omitted column is None
    assert!(users[0].invited_by.is_none());
});

// -- Partial columns with relations --
postgres_test!(query_columns_with_relations, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post, .. } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );

    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let alice_id = all_users[0].id;

    drizzle_exec!(
        db.insert(post)
            .values([
                InsertPost::new("Post 1", true).with_author_id(alice_id),
                InsertPost::new("Post 2", true).with_author_id(alice_id),
            ])
            => execute
    );

    // Partial columns on base, full relations
    let users = drizzle_exec!(
        db.query(complex)
            .columns(complex.select_columns().id().name())
            .with(complex.posts())
            .find_many()
    );

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name.as_deref(), Some("Alice"));
    assert!(users[0].invited_by.is_none()); // not selected
    assert_eq!(users[0].posts().len(), 2);
    // Relations are full SelectModel (not partial)
    assert_eq!(users[0].posts()[0].title, "Post 1");
});

// -- Partial columns on a relation --
postgres_test!(query_relation_columns, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post, .. } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );

    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let alice_id = all_users[0].id;

    drizzle_exec!(
        db.insert(post)
            .values([
                InsertPost::new("Post 1", true).with_author_id(alice_id),
                InsertPost::new("Post 2", true).with_author_id(alice_id),
            ])
            => execute
    );

    // Full base, partial columns on relation
    let users = drizzle_exec!(
        db.query(complex)
            .with(complex.posts().columns(post.select_columns().id().title()))
            .find_many()
    );

    assert_eq!(users.len(), 1);
    // Base is full SelectModel
    assert_eq!(users[0].name, "Alice");
    // Relation is PartialSelectModel
    assert_eq!(users[0].posts().len(), 2);
    assert!(users[0].posts()[0].id.is_some());
    assert_eq!(users[0].posts()[0].title.as_deref(), Some("Post 1"));
    // author_id not selected
    assert!(users[0].posts()[0].author_id.is_none());
});

// -- find_first with partial columns --
postgres_test!(query_columns_find_first, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema {
        complex, post: _, ..
    } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );

    let user = drizzle_exec!(
        db.query(complex)
            .columns(complex.select_columns().name())
            .find_first()
    );

    assert!(user.is_some());
    let user = user.unwrap();
    assert_eq!(user.name.as_deref(), Some("Alice"));
    assert!(user.id.is_none()); // not selected
});
