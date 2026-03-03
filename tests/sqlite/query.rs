#![cfg(all(
    any(feature = "rusqlite", feature = "turso", feature = "libsql"),
    feature = "query",
    feature = "uuid"
))]

use drizzle::core::expr::{eq, gt};
use drizzle::core::{asc, desc};
use drizzle::sqlite::prelude::*;
use drizzle_macros::sqlite_test;
use uuid::Uuid;

use crate::common::schema::sqlite::{
    Category, Comment, Complex, InsertCategory, InsertComment, InsertComplex, InsertPost,
    InsertPostCategory, InsertReply, Post, PostCategory, Reply, Role, SelectCategory,
    SelectComment, SelectComplex, SelectPost,
};

// Import generated relation accessor traits from the common schema.
// These are needed because the table definitions live in a different module.
#[allow(unused_imports)]
use crate::common::schema::sqlite::{
    __Category_ViaPostCategory_Posts_RelAccessor, __ColumnsAccessor_Complex,
    __ColumnsAccessor_Post, __Comment_Replies_RelAccessor, __Complex_InvitedBy_RelAccessor,
    __Complex_Posts_RelAccessor, __Post_Author_RelAccessor, __Post_Comments_RelAccessor,
    __Post_ViaPostCategory_Categories_RelAccessor, __QueryAccess_Category_ViaPostCategory_Posts,
    __QueryAccess_Comment_Replies, __QueryAccess_Complex_InvitedBy, __QueryAccess_Complex_Posts,
    __QueryAccess_Post_Author, __QueryAccess_Post_Comments,
    __QueryAccess_Post_ViaPostCategory_Categories, ComplexId, ComplexWithInvitedBy,
    ComplexWithPosts,
};

// =============================================================================
// Schemas for different test scenarios
// =============================================================================

#[derive(SQLiteSchema)]
struct ComplexPostQuerySchema {
    complex: Complex,
    post: Post,
}

#[derive(SQLiteSchema)]
struct FullQuerySchema {
    complex: Complex,
    post: Post,
    comment: Comment,
}

#[derive(SQLiteSchema)]
struct DeepQuerySchema {
    complex: Complex,
    post: Post,
    comment: Comment,
    reply: Reply,
}

// =============================================================================
// Tests
// =============================================================================

// -- Basic find_many without relations --
sqlite_test!(query_find_many_no_relations, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post: _ } = schema;

    // Insert users
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
sqlite_test!(query_find_first, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post: _ } = schema;

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
sqlite_test!(query_find_first_empty, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post: _ } = schema;

    let user = drizzle_exec!(db.query(complex).find_first());
    assert!(user.is_none());
});

// -- with limit --
sqlite_test!(query_with_limit, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post: _ } = schema;

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
sqlite_test!(query_reverse_relation_many, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post } = schema;

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

// -- Forward relation: Post -> Author (OptionalOne) --
sqlite_test!(query_forward_relation_one, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post } = schema;

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
sqlite_test!(query_forward_optional_one, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post: _ } = schema;

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
sqlite_test!(query_nested_relations, FullQuerySchema, {
    let FullQuerySchema {
        complex,
        post,
        comment,
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
sqlite_test!(query_multiple_relations, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post } = schema;

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
sqlite_test!(query_empty_many_relation, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post: _ } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );

    let users = drizzle_exec!(db.query(complex).with(complex.posts()).find_many());

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].posts().len(), 0);
});

// -- Typed WHERE on root query --
sqlite_test!(query_where_typed, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post: _ } = schema;

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
sqlite_test!(query_order_by_typed, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post: _ } = schema;

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

// -- Typed WHERE on relation subquery --
sqlite_test!(query_relation_where_typed, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post } = schema;

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

    // Only include posts with title > "Post A" (should exclude "Post A")
    let users = drizzle_exec!(
        db.query(complex)
            .with(complex.posts().r#where(gt(post.title, "Post A")))
            .find_many()
    );

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].posts().len(), 2);
});

// -- Typed ORDER BY + LIMIT on relation subquery --
sqlite_test!(query_relation_order_limit_typed, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post } = schema;

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

// -- Forward relation with NULL FK --
sqlite_test!(query_forward_relation_null_fk, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );

    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let alice_id = all_users[0].id;

    // Post with author
    drizzle_exec!(
        db.insert(post)
            .values([InsertPost::new("With Author", true).with_author_id(alice_id)])
            => execute
    );
    // Post without author (NULL FK)
    drizzle_exec!(
        db.insert(post)
            .values([InsertPost::new("No Author", true)])
            => execute
    );

    let posts = drizzle_exec!(
        db.query(post)
            .with(post.author())
            .order_by(asc(post.title))
            .find_many()
    );

    assert_eq!(posts.len(), 2);
    // "No Author" comes first alphabetically
    assert!(posts[0].author().is_none());
    assert!(posts[1].author().is_some());
    assert_eq!(posts[1].author().as_ref().unwrap().name, "Alice");
});

// -- Combined root WHERE + relation WHERE (tests param ordering) --
sqlite_test!(
    query_root_and_relation_where_combined,
    ComplexPostQuerySchema,
    {
        let ComplexPostQuerySchema { complex, post } = schema;

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
                    InsertPost::new("Alice Draft", false).with_author_id(alice_id),
                    InsertPost::new("Alice Published", true).with_author_id(alice_id),
                    InsertPost::new("Bob Post", true).with_author_id(bob_id),
                ])
                => execute
        );

        // Root WHERE filters to Alice, relation WHERE filters to published posts
        let users = drizzle_exec!(
            db.query(complex)
                .with(complex.posts().r#where(eq(post.published, true)))
                .r#where(eq(complex.name, "Alice"))
                .find_many()
        );

        assert_eq!(users.len(), 1);
        assert_eq!(users[0].name, "Alice");
        assert_eq!(users[0].posts().len(), 1);
        assert_eq!(users[0].posts()[0].title, "Alice Published");
    }
);

// =============================================================================
// View support
// =============================================================================

#[SQLiteView(DEFINITION = "SELECT id, title, author_id FROM posts")]
struct PostView {
    id: Uuid,
    title: String,
    author_id: Option<Uuid>,
}

#[derive(SQLiteSchema)]
struct ViewSchema {
    complex: Complex,
    post: Post,
    post_view: PostView,
}

// -- Basic view query without relations --
sqlite_test!(query_view_find_many, ViewSchema, {
    let ViewSchema {
        complex,
        post,
        post_view,
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
sqlite_test!(query_view_find_first, ViewSchema, {
    let ViewSchema {
        complex,
        post,
        post_view,
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

    let post_result = drizzle_exec!(
        db.query(post_view)
            .order_by(asc(post_view.title))
            .find_first()
    );
    assert!(post_result.is_some());
    assert_eq!(post_result.unwrap().title, "First Post");
});

// -- View with WHERE and ORDER BY --
sqlite_test!(query_view_where_order, ViewSchema, {
    let ViewSchema {
        complex,
        post,
        post_view,
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
#[SQLiteView(DEFINITION = "SELECT id, title, author_id FROM posts")]
struct PostViewFk {
    id: Uuid,
    title: String,
    #[column(REFERENCES = Complex::id)]
    author_id: Option<Uuid>,
}

#[derive(SQLiteSchema)]
struct ViewFkSchema {
    complex: Complex,
    post: Post,
    post_view_fk: PostViewFk,
}

// -- View with forward relation (view -> table) --
sqlite_test!(query_view_with_forward_relation, ViewFkSchema, {
    let ViewFkSchema {
        complex,
        post,
        post_view_fk,
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
sqlite_test!(query_combo_tables_and_views, ViewFkSchema, {
    let ViewFkSchema {
        complex,
        post,
        post_view_fk,
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
sqlite_test!(query_deep_nested_complex, DeepQuerySchema, {
    let DeepQuerySchema {
        complex,
        post,
        comment,
        reply,
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
                        .with(comment.replies()),
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
    assert_eq!(alice_posts[1].comments()[0].replies().len(), 0);

    // Alice Draft: 3 comments ordered by body ASC
    let draft_comments = alice_posts[2].comments();
    assert_eq!(draft_comments.len(), 3);
    assert_eq!(draft_comments[0].body, "Great draft!");
    assert_eq!(draft_comments[1].body, "Love this");
    assert_eq!(draft_comments[2].body, "Needs work");
    // "Great draft!" has 1 reply
    assert_eq!(draft_comments[0].replies().len(), 1);
    assert_eq!(draft_comments[0].replies()[0].text, "Thanks!");
    // "Love this" has 0 replies
    assert_eq!(draft_comments[1].replies().len(), 0);
    // "Needs work" has 1 reply
    assert_eq!(draft_comments[2].replies().len(), 1);
    assert_eq!(draft_comments[2].replies()[0].text, "Will revise");

    // -- Bob: invited by Alice, 1 post with 1 comment with 1 reply --
    assert!(users[1].invited_by().is_some());
    assert_eq!(users[1].invited_by().as_ref().unwrap().name, "Alice");
    assert_eq!(users[1].posts().len(), 1);
    assert_eq!(users[1].posts()[0].title, "Bob First Post");
    assert_eq!(users[1].posts()[0].comments().len(), 1);
    assert_eq!(users[1].posts()[0].comments()[0].body, "Welcome Bob!");
    assert_eq!(users[1].posts()[0].comments()[0].replies().len(), 1);
    assert_eq!(
        users[1].posts()[0].comments()[0].replies()[0].text,
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

sqlite_test!(query_type_alias_in_fn_signature, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post } = schema;

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
sqlite_test!(query_with_limit_offset, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post: _ } = schema;

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
sqlite_test!(query_relation_limit_offset, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post } = schema;

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
sqlite_test!(query_columns_whitelist, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post: _ } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([
                InsertComplex::new("Alice", true, Role::User),
                InsertComplex::new("Bob", true, Role::User),
            ])
            => execute
    );

    // Select only id and name (omitting invited_by)
    let users = drizzle_exec!(
        db.query(complex)
            .columns(complex.columns().id().name())
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
sqlite_test!(query_omit_blacklist, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post: _ } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );

    // Omit invited_by — should still return id and name
    let users = drizzle_exec!(
        db.query(complex)
            .omit(complex.columns().invited_by())
            .find_many()
    );

    assert_eq!(users.len(), 1);
    assert!(users[0].id.is_some());
    assert_eq!(users[0].name.as_deref(), Some("Alice"));
    // Omitted column is None
    assert!(users[0].invited_by.is_none());
});

// -- Partial columns with relations --
sqlite_test!(query_columns_with_relations, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post } = schema;

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
            .columns(complex.columns().id().name())
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
sqlite_test!(query_relation_columns, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post } = schema;

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
            .with(complex.posts().columns(post.columns().id().title()))
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
sqlite_test!(query_columns_find_first, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post: _ } = schema;

    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );

    let user = drizzle_exec!(
        db.query(complex)
            .columns(complex.columns().name())
            .find_first()
    );

    assert!(user.is_some());
    let user = user.unwrap();
    assert_eq!(user.name.as_deref(), Some("Alice"));
    assert!(user.id.is_none()); // not selected
});

// =============================================================================
// .first() on relations
// =============================================================================

// -- .first() limits relation to at most 1 element --
sqlite_test!(query_first_limits_to_one, ComplexPostQuerySchema, {
    let ComplexPostQuerySchema { complex, post } = schema;

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
                InsertPost::new("Post 3", true).with_author_id(alice_id),
            ])
            => execute
    );

    let users = drizzle_exec!(db.query(complex).with(complex.posts().first()).find_many());

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].posts().len(), 1);
});

// =============================================================================
// Many-to-many relations
// =============================================================================

#[derive(SQLiteSchema)]
struct M2MQuerySchema {
    complex: Complex,
    post: Post,
    category: Category,
    post_category: PostCategory,
}

// -- basic m2m: post.categories() returns categories through junction --
sqlite_test!(query_many_to_many_basic, M2MQuerySchema, {
    let M2MQuerySchema {
        complex,
        post,
        category,
        post_category,
    } = schema;

    // Insert author
    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );
    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let alice_id = all_users[0].id;

    // Insert post
    drizzle_exec!(
        db.insert(post)
            .values([InsertPost::new("My Post", true).with_author_id(alice_id)])
            => execute
    );
    let all_posts: Vec<SelectPost> = drizzle_exec!(db.select(()).from(post) => all);
    let post_id = all_posts[0].id;

    // Insert categories
    drizzle_exec!(
        db.insert(category)
            .values([
                InsertCategory::new("Tech"),
                InsertCategory::new("Science"),
            ])
            => execute
    );
    let all_cats: Vec<SelectCategory> = drizzle_exec!(db.select(()).from(category) => all);

    // Link post to both categories
    drizzle_exec!(
        db.insert(post_category)
            .values([
                InsertPostCategory::new(post_id, all_cats[0].id),
                InsertPostCategory::new(post_id, all_cats[1].id),
            ])
            => execute
    );

    // Query posts with their categories through the junction
    let posts = drizzle_exec!(db.query(post).with(post.categories()).find_many());

    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0].title, "My Post");
    assert_eq!(posts[0].categories().len(), 2);
    let cat_names: Vec<&str> = posts[0]
        .categories()
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert!(cat_names.contains(&"Tech"));
    assert!(cat_names.contains(&"Science"));
});

// -- reverse m2m: category.posts() returns posts through junction --
sqlite_test!(query_many_to_many_reverse, M2MQuerySchema, {
    let M2MQuerySchema {
        complex,
        post,
        category,
        post_category,
    } = schema;

    // Insert author
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
                InsertPost::new("Post A", true).with_author_id(alice_id),
                InsertPost::new("Post B", true).with_author_id(alice_id),
            ])
            => execute
    );
    let all_posts: Vec<SelectPost> = drizzle_exec!(db.select(()).from(post) => all);

    // Insert category
    drizzle_exec!(
        db.insert(category)
            .values([InsertCategory::new("Tech")])
            => execute
    );
    let all_cats: Vec<SelectCategory> = drizzle_exec!(db.select(()).from(category) => all);
    let cat_id = all_cats[0].id;

    // Link category to both posts
    drizzle_exec!(
        db.insert(post_category)
            .values([
                InsertPostCategory::new(all_posts[0].id, cat_id),
                InsertPostCategory::new(all_posts[1].id, cat_id),
            ])
            => execute
    );

    // Query categories with their posts
    let cats = drizzle_exec!(db.query(category).with(category.posts()).find_many());

    assert_eq!(cats.len(), 1);
    assert_eq!(cats[0].name, "Tech");
    assert_eq!(cats[0].posts().len(), 2);
    let post_titles: Vec<&str> = cats[0].posts().iter().map(|p| p.title.as_str()).collect();
    assert!(post_titles.contains(&"Post A"));
    assert!(post_titles.contains(&"Post B"));
});

// -- m2m with no associations returns empty vec --
sqlite_test!(query_many_to_many_empty, M2MQuerySchema, {
    let M2MQuerySchema {
        complex,
        post,
        category: _,
        post_category: _,
    } = schema;

    // Insert author and post with no category links
    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );
    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let alice_id = all_users[0].id;

    drizzle_exec!(
        db.insert(post)
            .values([InsertPost::new("Lonely Post", true).with_author_id(alice_id)])
            => execute
    );

    let posts = drizzle_exec!(db.query(post).with(post.categories()).find_many());

    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0].categories().len(), 0);
});

// -- m2m with limit --
sqlite_test!(query_many_to_many_with_limit, M2MQuerySchema, {
    let M2MQuerySchema {
        complex,
        post,
        category,
        post_category,
    } = schema;

    // Insert author
    drizzle_exec!(
        db.insert(complex)
            .values([InsertComplex::new("Alice", true, Role::User)])
            => execute
    );
    let all_users: Vec<SelectComplex> = drizzle_exec!(db.select(()).from(complex) => all);
    let alice_id = all_users[0].id;

    // Insert post
    drizzle_exec!(
        db.insert(post)
            .values([InsertPost::new("My Post", true).with_author_id(alice_id)])
            => execute
    );
    let all_posts: Vec<SelectPost> = drizzle_exec!(db.select(()).from(post) => all);
    let post_id = all_posts[0].id;

    // Insert 3 categories and link all to the post
    drizzle_exec!(
        db.insert(category)
            .values([
                InsertCategory::new("A"),
                InsertCategory::new("B"),
                InsertCategory::new("C"),
            ])
            => execute
    );
    let all_cats: Vec<SelectCategory> = drizzle_exec!(db.select(()).from(category) => all);

    drizzle_exec!(
        db.insert(post_category)
            .values([
                InsertPostCategory::new(post_id, all_cats[0].id),
                InsertPostCategory::new(post_id, all_cats[1].id),
                InsertPostCategory::new(post_id, all_cats[2].id),
            ])
            => execute
    );

    let posts = drizzle_exec!(db.query(post).with(post.categories().limit(2)).find_many());

    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0].categories().len(), 2);
});
