/// Relational Query API coverage shared by every dialect.
///
/// The SQLite and PostgreSQL suites used to carry near-identical copies of
/// these tests on their UUID-keyed `Complex`/`Post` fixtures; this suite runs
/// the same scenarios on integer-keyed tables so the assertions can pin exact
/// ids, and it runs on MySQL as well. Each dialect keeps a couple of
/// UUID-keyed scenarios of its own.
macro_rules! shared_relational_api_suite {
    (
        $dialect:ident,
        $table:ident,
        $view:ident,
        $schema:ident,
        $integer:path,
        $transaction_config:expr
    ) => {
        mod shared_relational_api {
            use super::*;
            use drizzle::core::expr::{eq, gt};
            use drizzle::core::{asc, desc};

            #[$table(NAME = "shared_api_authors")]
            struct SharedApiAuthor {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
                /// Self-referential optional forward relation (`invited_by()`)
                /// whose auto-disambiguated reverse is
                /// `invited_by_shared_api_authors()`.
                #[column(REFERENCES = SharedApiAuthor::id)]
                invited_by: Option<i32>,
            }

            #[$table(NAME = "shared_api_posts")]
            struct SharedApiPost {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                title: String,
                published: bool,
                /// Nullable FK: `author()` is an optional forward relation.
                #[column(REFERENCES = SharedApiAuthor::id, RELATION = "posts")]
                author_id: Option<i32>,
            }

            #[$table(NAME = "shared_api_comments")]
            struct SharedApiComment {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                body: String,
                #[column(REFERENCES = SharedApiPost::id, RELATION = "comments")]
                post_id: i32,
            }

            #[$table(NAME = "shared_api_replies")]
            struct SharedApiReply {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                text: String,
                #[column(REFERENCES = SharedApiComment::id, RELATION = "replies")]
                comment_id: i32,
            }

            #[$table(NAME = "shared_api_categories")]
            struct SharedApiCategory {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
            }

            /// Junction table: gives `posts.shared_api_categories()` and
            /// `categories.shared_api_posts()`.
            #[$table(NAME = "shared_api_post_categories")]
            struct SharedApiPostCategory {
                #[column(REFERENCES = SharedApiPost::id)]
                post_id: i32,
                #[column(REFERENCES = SharedApiCategory::id)]
                category_id: i32,
            }

            #[$table(NAME = "shared_api_articles")]
            struct SharedApiArticle {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                title: String,
                /// Reverse on the author: `authored()` via the explicit name.
                #[column(REFERENCES = SharedApiAuthor::id, RELATION = "authored")]
                author_id: i32,
                /// Reverse on the author: `editor_shared_api_articles()` via
                /// auto-disambiguation.
                #[column(REFERENCES = SharedApiAuthor::id)]
                editor_id: Option<i32>,
            }

            /// A view that carries a forward relation of its own.
            #[$view(
                NAME = "shared_api_post_headlines",
                DEFINITION = "SELECT id, title, author_id FROM shared_api_posts"
            )]
            struct SharedApiPostHeadline {
                id: i32,
                title: String,
                #[column(REFERENCES = SharedApiAuthor::id)]
                author_id: Option<i32>,
            }

            #[derive($schema)]
            struct SharedApiSchema {
                authors: SharedApiAuthor,
                posts: SharedApiPost,
                comments: SharedApiComment,
                replies: SharedApiReply,
                categories: SharedApiCategory,
                post_categories: SharedApiPostCategory,
                articles: SharedApiArticle,
                headlines: SharedApiPostHeadline,
            }

            const ALICE: i32 = 1;
            const BOB: i32 = 2;
            const CHARLIE: i32 = 3;
            const DAVE: i32 = 4;

            /// An author with its id set (the insert model is a typestate, so
            /// the helper names the fields it has filled in).
            type AuthorRow = InsertSharedApiAuthor<
                'static,
                (
                    SharedApiAuthorIdSet,
                    SharedApiAuthorNameSet,
                    SharedApiAuthorInvitedByNotSet,
                ),
            >;

            fn author(id: i32, name: &'static str) -> AuthorRow {
                InsertSharedApiAuthor::new(name).with_id(id)
            }

            type PostRow = InsertSharedApiPost<
                'static,
                (
                    SharedApiPostIdSet,
                    SharedApiPostTitleSet,
                    SharedApiPostPublishedSet,
                    SharedApiPostAuthorIdSet,
                ),
            >;

            fn post(id: i32, title: &'static str, published: bool, author_id: i32) -> PostRow {
                InsertSharedApiPost::new(title, published)
                    .with_id(id)
                    .with_author_id(author_id)
            }

            // ---------------------------------------------------------------- roots

            #[drizzle::test($dialect)]
            fn find_many_and_find_first(db: &mut TestDb<SharedApiSchema>) {
                let SharedApiSchema { authors, .. } = schema;

                let missing = db.query(authors).find_first();
                assert!(missing.is_none());

                db.insert(authors)
                    .values([author(BOB, "Bob"), author(ALICE, "Alice")])
                    .execute();

                let all = db.query(authors).order_by(asc(authors.name)).find_many();
                assert_eq!(all.len(), 2);
                assert_eq!(all[0].name, "Alice");
                assert_eq!(all[1].name, "Bob");

                let first = db.query(authors).order_by(asc(authors.name)).find_first();
                assert_eq!(first.map(|row| row.name).as_deref(), Some("Alice"));
            }

            #[drizzle::test($dialect)]
            fn root_where_order_limit_offset(db: &mut TestDb<SharedApiSchema>) {
                let SharedApiSchema { authors, .. } = schema;
                db.insert(authors)
                    .values([
                        author(CHARLIE, "Charlie"),
                        author(ALICE, "Alice"),
                        author(DAVE, "Dave"),
                        author(BOB, "Bob"),
                    ])
                    .execute();

                let bob = db
                    .query(authors)
                    .r#where(eq(authors.name, "Bob"))
                    .find_many();
                assert_eq!(bob.len(), 1);
                assert_eq!(bob[0].id, BOB);

                let ascending = db.query(authors).order_by(asc(authors.name)).find_many();
                assert_eq!(
                    ascending
                        .iter()
                        .map(|row| row.name.as_str())
                        .collect::<Vec<_>>(),
                    ["Alice", "Bob", "Charlie", "Dave"]
                );
                let descending = db.query(authors).order_by(desc(authors.name)).find_many();
                assert_eq!(descending[0].name, "Dave");

                let limited = db.query(authors).limit(2).find_many();
                assert_eq!(limited.len(), 2);

                let page = db
                    .query(authors)
                    .order_by(asc(authors.name))
                    .limit(2)
                    .offset(1)
                    .find_many();
                assert_eq!(
                    page.iter().map(|row| row.name.as_str()).collect::<Vec<_>>(),
                    ["Bob", "Charlie"]
                );
            }

            // ------------------------------------------------------------ relations

            #[drizzle::test($dialect)]
            fn reverse_relation_collects_children(db: &mut TestDb<SharedApiSchema>) {
                let SharedApiSchema { authors, posts, .. } = schema;
                db.insert(authors)
                    .values([author(ALICE, "Alice"), author(BOB, "Bob")])
                    .execute();
                db.insert(posts)
                    .values([
                        post(10, "Alice Post 1", true, ALICE),
                        post(11, "Alice Post 2", true, ALICE),
                        post(12, "Bob Post 1", true, BOB),
                    ])
                    .execute();

                let users = db
                    .query(authors)
                    .with(authors.posts().order_by(asc(posts.id)))
                    .order_by(asc(authors.id))
                    .find_many();
                assert_eq!(users.len(), 2);
                assert_eq!(users[0].posts.len(), 2);
                assert_eq!(users[0].posts[0].title, "Alice Post 1");
                assert_eq!(users[0].posts[1].title, "Alice Post 2");
                assert_eq!(users[1].posts.len(), 1);
                assert_eq!(users[1].posts[0].title, "Bob Post 1");

                let lonely = db
                    .query(authors)
                    .r#where(eq(authors.id, BOB))
                    .with(authors.posts().r#where(eq(posts.published, false)))
                    .find_first()
                    .unwrap();
                assert!(lonely.posts.is_empty());
            }

            #[drizzle::test($dialect)]
            fn forward_relations_are_optional_when_the_key_is_nullable(
                db: &mut TestDb<SharedApiSchema>,
            ) {
                let SharedApiSchema { authors, posts, .. } = schema;
                db.insert(authors).value(author(ALICE, "Alice")).execute();
                db.insert(authors)
                    .value(author(BOB, "Bob").with_invited_by(ALICE))
                    .execute();
                db.insert(posts)
                    .value(post(10, "With Author", true, ALICE))
                    .execute();
                db.insert(posts)
                    .value(InsertSharedApiPost::new("No Author", true).with_id(11))
                    .execute();

                let by_title = db
                    .query(posts)
                    .with(posts.author())
                    .order_by(asc(posts.title))
                    .find_many();
                assert_eq!(by_title.len(), 2);
                assert!(by_title[0].author.is_none());
                assert_eq!(
                    by_title[1]
                        .author
                        .as_ref()
                        .map(|author| author.name.as_str()),
                    Some("Alice")
                );

                let users = db
                    .query(authors)
                    .with(authors.invited_by())
                    .order_by(asc(authors.id))
                    .find_many();
                assert!(users[0].invited_by.is_none());
                assert_eq!(
                    users[1]
                        .invited_by
                        .as_ref()
                        .map(|inviter| inviter.name.as_str()),
                    Some("Alice")
                );
            }

            #[drizzle::test($dialect)]
            fn sibling_relations_load_independently(db: &mut TestDb<SharedApiSchema>) {
                let SharedApiSchema { authors, posts, .. } = schema;
                db.insert(authors).value(author(ALICE, "Alice")).execute();
                db.insert(authors)
                    .value(author(BOB, "Bob").with_invited_by(ALICE))
                    .execute();
                db.insert(posts)
                    .value(post(10, "Bob's Post", true, BOB))
                    .execute();

                let users = db
                    .query(authors)
                    .with(authors.posts())
                    .with(authors.invited_by())
                    .order_by(asc(authors.id))
                    .find_many();
                assert_eq!(users.len(), 2);
                assert!(users[0].posts.is_empty());
                assert!(users[0].invited_by.is_none());
                assert_eq!(users[1].posts.len(), 1);
                assert_eq!(users[1].posts[0].title, "Bob's Post");
                assert_eq!(
                    users[1]
                        .invited_by
                        .as_ref()
                        .map(|inviter| inviter.name.as_str()),
                    Some("Alice")
                );
            }

            #[drizzle::test($dialect)]
            fn relation_where_order_limit_offset_and_first(db: &mut TestDb<SharedApiSchema>) {
                let SharedApiSchema { authors, posts, .. } = schema;
                db.insert(authors)
                    .values([author(ALICE, "Alice"), author(BOB, "Bob")])
                    .execute();
                db.insert(posts)
                    .values([
                        post(10, "CCC", true, ALICE),
                        post(11, "AAA", false, ALICE),
                        post(12, "DDD", true, ALICE),
                        post(13, "BBB", true, ALICE),
                        post(14, "Bob Post", true, BOB),
                    ])
                    .execute();

                let after_a = db
                    .query(authors)
                    .r#where(eq(authors.id, ALICE))
                    .with(authors.posts().r#where(gt(posts.title, "AAA")))
                    .find_first()
                    .unwrap();
                assert_eq!(after_a.posts.len(), 3);

                let top_two = db
                    .query(authors)
                    .r#where(eq(authors.id, ALICE))
                    .with(authors.posts().order_by(desc(posts.title)).limit(2))
                    .find_first()
                    .unwrap();
                assert_eq!(
                    top_two
                        .posts
                        .iter()
                        .map(|post| post.title.as_str())
                        .collect::<Vec<_>>(),
                    ["DDD", "CCC"]
                );

                let page = db
                    .query(authors)
                    .r#where(eq(authors.id, ALICE))
                    .with(
                        authors
                            .posts()
                            .order_by(asc(posts.title))
                            .limit(2)
                            .offset(1),
                    )
                    .find_first()
                    .unwrap();
                assert_eq!(
                    page.posts
                        .iter()
                        .map(|post| post.title.as_str())
                        .collect::<Vec<_>>(),
                    ["BBB", "CCC"]
                );

                // Root WHERE and relation WHERE keep their parameters apart.
                let published_only = db
                    .query(authors)
                    .with(authors.posts().r#where(eq(posts.published, true)))
                    .r#where(eq(authors.name, "Alice"))
                    .find_many();
                assert_eq!(published_only.len(), 1);
                assert_eq!(published_only[0].posts.len(), 3);

                let first = db
                    .query(authors)
                    .r#where(eq(authors.id, ALICE))
                    .with(authors.posts().first())
                    .find_first()
                    .unwrap();
                assert_eq!(first.posts.len(), 1);
            }

            #[drizzle::test($dialect)]
            fn deep_nesting_with_ordering_and_limits(db: &mut TestDb<SharedApiSchema>) {
                let SharedApiSchema {
                    authors,
                    posts,
                    comments,
                    replies,
                    ..
                } = schema;
                db.insert(authors)
                    .values([author(ALICE, "Alice"), author(DAVE, "Dave")])
                    .execute();
                db.insert(authors)
                    .values([
                        author(BOB, "Bob").with_invited_by(ALICE),
                        author(CHARLIE, "Charlie").with_invited_by(ALICE),
                    ])
                    .execute();
                db.insert(posts)
                    .values([
                        post(10, "Alice Draft", true, ALICE),
                        post(11, "Alice Thoughts", true, ALICE),
                        post(12, "Alice Update", true, ALICE),
                        post(13, "Alice Announcement", true, ALICE),
                        post(14, "Bob First Post", true, BOB),
                    ])
                    .execute();
                db.insert(comments)
                    .values([
                        InsertSharedApiComment::new("Great draft!", 10).with_id(20),
                        InsertSharedApiComment::new("Needs work", 10).with_id(21),
                        InsertSharedApiComment::new("Love this", 10).with_id(22),
                        InsertSharedApiComment::new("Interesting thoughts", 11).with_id(23),
                        InsertSharedApiComment::new("Welcome Bob!", 14).with_id(24),
                    ])
                    .execute();
                db.insert(replies)
                    .values([
                        InsertSharedApiReply::new("Thanks!", 20).with_id(30),
                        InsertSharedApiReply::new("Will revise", 21).with_id(31),
                        InsertSharedApiReply::new("Glad to be here", 24).with_id(32),
                    ])
                    .execute();

                // Four levels deep, sibling relations on the root, ORDER BY and
                // LIMIT on a nested Many relation.
                let users = db
                    .query(authors)
                    .with(
                        authors.posts().order_by(desc(posts.title)).limit(3).with(
                            posts
                                .comments()
                                .order_by(asc(comments.body))
                                .with(comments.replies()),
                        ),
                    )
                    .with(authors.invited_by())
                    .order_by(asc(authors.name))
                    .find_many();

                assert_eq!(
                    users
                        .iter()
                        .map(|row| row.name.as_str())
                        .collect::<Vec<_>>(),
                    ["Alice", "Bob", "Charlie", "Dave"]
                );

                let alice = &users[0];
                assert!(alice.invited_by.is_none());
                assert_eq!(
                    alice
                        .posts
                        .iter()
                        .map(|post| post.title.as_str())
                        .collect::<Vec<_>>(),
                    ["Alice Update", "Alice Thoughts", "Alice Draft"]
                );
                assert!(alice.posts[0].comments.is_empty());
                assert_eq!(alice.posts[1].comments.len(), 1);
                assert!(alice.posts[1].comments[0].replies.is_empty());
                let draft = &alice.posts[2].comments;
                assert_eq!(
                    draft.iter().map(|c| c.body.as_str()).collect::<Vec<_>>(),
                    ["Great draft!", "Love this", "Needs work"]
                );
                assert_eq!(draft[0].replies[0].text, "Thanks!");
                assert!(draft[1].replies.is_empty());
                assert_eq!(draft[2].replies[0].text, "Will revise");

                let bob = &users[1];
                assert_eq!(
                    bob.invited_by.as_ref().map(|inviter| inviter.name.as_str()),
                    Some("Alice")
                );
                assert_eq!(bob.posts.len(), 1);
                assert_eq!(bob.posts[0].comments[0].replies[0].text, "Glad to be here");

                assert!(users[2].posts.is_empty());
                assert!(users[2].invited_by.is_some());
                assert!(users[3].posts.is_empty());
                assert!(users[3].invited_by.is_none());
            }

            #[drizzle::test($dialect)]
            fn many_to_many_through_a_junction(db: &mut TestDb<SharedApiSchema>) {
                let SharedApiSchema {
                    authors,
                    posts,
                    categories,
                    post_categories,
                    ..
                } = schema;
                db.insert(authors).value(author(ALICE, "Alice")).execute();
                db.insert(posts)
                    .values([
                        post(10, "Post A", true, ALICE),
                        post(11, "Post B", true, ALICE),
                        post(12, "Lonely Post", true, ALICE),
                    ])
                    .execute();
                db.insert(categories)
                    .values([
                        InsertSharedApiCategory::new("Tech").with_id(40),
                        InsertSharedApiCategory::new("Science").with_id(41),
                        InsertSharedApiCategory::new("Art").with_id(42),
                    ])
                    .execute();
                db.insert(post_categories)
                    .values([
                        InsertSharedApiPostCategory::new(10, 40),
                        InsertSharedApiPostCategory::new(10, 41),
                        InsertSharedApiPostCategory::new(10, 42),
                        InsertSharedApiPostCategory::new(11, 40),
                    ])
                    .execute();

                let tagged = db
                    .query(posts)
                    .with(posts.shared_api_categories().order_by(asc(categories.id)))
                    .order_by(asc(posts.id))
                    .find_many();
                assert_eq!(tagged.len(), 3);
                assert_eq!(
                    tagged[0]
                        .shared_api_categories
                        .iter()
                        .map(|category| category.name.as_str())
                        .collect::<Vec<_>>(),
                    ["Tech", "Science", "Art"]
                );
                assert_eq!(tagged[1].shared_api_categories.len(), 1);
                assert!(tagged[2].shared_api_categories.is_empty());

                let reverse = db
                    .query(categories)
                    .r#where(eq(categories.id, 40))
                    .with(categories.shared_api_posts().order_by(asc(posts.id)))
                    .find_first()
                    .unwrap();
                assert_eq!(
                    reverse
                        .shared_api_posts
                        .iter()
                        .map(|post| post.title.as_str())
                        .collect::<Vec<_>>(),
                    ["Post A", "Post B"]
                );

                let limited = db
                    .query(posts)
                    .r#where(eq(posts.id, 10))
                    .with(posts.shared_api_categories().limit(2))
                    .find_first()
                    .unwrap();
                assert_eq!(limited.shared_api_categories.len(), 2);
            }

            #[drizzle::test($dialect)]
            fn multiple_foreign_keys_to_one_table_are_disambiguated(
                db: &mut TestDb<SharedApiSchema>,
            ) {
                let SharedApiSchema {
                    authors, articles, ..
                } = schema;
                db.insert(authors).value(author(ALICE, "Alice")).execute();
                db.insert(authors)
                    .values([
                        author(BOB, "Bob").with_invited_by(ALICE),
                        author(CHARLIE, "Charlie").with_invited_by(ALICE),
                    ])
                    .execute();
                db.insert(articles)
                    .value(
                        InsertSharedApiArticle::new("Draft A", ALICE)
                            .with_id(50)
                            .with_editor_id(BOB),
                    )
                    .execute();
                db.insert(articles)
                    .value(InsertSharedApiArticle::new("Draft B", BOB).with_id(51))
                    .execute();

                // `RELATION = "authored"` names the reverse accessor.
                let by_author = db
                    .query(authors)
                    .with(authors.authored())
                    .order_by(asc(authors.id))
                    .find_many();
                assert_eq!(by_author[0].authored[0].title, "Draft A");
                assert_eq!(by_author[1].authored[0].title, "Draft B");
                assert!(by_author[2].authored.is_empty());

                // The second FK to the same table gets the column-prefixed name.
                let by_editor = db
                    .query(authors)
                    .with(authors.editor_shared_api_articles())
                    .order_by(asc(authors.id))
                    .find_many();
                assert!(by_editor[0].editor_shared_api_articles.is_empty());
                assert_eq!(by_editor[1].editor_shared_api_articles[0].title, "Draft A");

                // Self-referential reverse: who did each author invite?
                let invitees = db
                    .query(authors)
                    .with(authors.invited_by_shared_api_authors())
                    .order_by(asc(authors.id))
                    .find_many();
                assert_eq!(invitees[0].invited_by_shared_api_authors.len(), 2);
                assert!(invitees[1].invited_by_shared_api_authors.is_empty());
            }

            // ---------------------------------------------------------------- views

            #[drizzle::test($dialect)]
            fn views_query_like_tables_and_carry_relations(db: &mut TestDb<SharedApiSchema>) {
                let SharedApiSchema {
                    authors,
                    posts,
                    headlines,
                    ..
                } = schema;
                db.insert(authors)
                    .values([author(ALICE, "Alice"), author(BOB, "Bob")])
                    .execute();
                db.insert(posts)
                    .values([
                        post(10, "Charlie Post", true, ALICE),
                        post(11, "Alpha Post", true, ALICE),
                        post(12, "Bravo Post", true, BOB),
                    ])
                    .execute();

                let titles = db
                    .query(headlines)
                    .order_by(asc(headlines.title))
                    .find_many();
                assert_eq!(
                    titles
                        .iter()
                        .map(|row| row.title.as_str())
                        .collect::<Vec<_>>(),
                    ["Alpha Post", "Bravo Post", "Charlie Post"]
                );

                let top = db
                    .query(headlines)
                    .order_by(desc(headlines.title))
                    .limit(2)
                    .find_many();
                assert_eq!(top[0].title, "Charlie Post");
                assert_eq!(top[1].title, "Bravo Post");

                let bravo = db
                    .query(headlines)
                    .r#where(eq(headlines.title, "Bravo Post"))
                    .find_first()
                    .unwrap();
                assert_eq!(bravo.id, 12);

                let with_authors = db
                    .query(headlines)
                    .with(headlines.author())
                    .order_by(asc(headlines.title))
                    .find_many();
                assert_eq!(
                    with_authors[0]
                        .author
                        .as_ref()
                        .map(|author| author.name.as_str()),
                    Some("Alice")
                );
                assert_eq!(
                    with_authors[1]
                        .author
                        .as_ref()
                        .map(|author| author.name.as_str()),
                    Some("Bob")
                );

                // Tables and views coexist in one schema.
                let users = db
                    .query(authors)
                    .with(authors.posts())
                    .order_by(asc(authors.name))
                    .find_many();
                assert_eq!(users[0].posts.len(), 2);
                assert_eq!(users[1].posts.len(), 1);
            }

            // -------------------------------------------------------- projections

            #[drizzle::test($dialect)]
            fn partial_columns_on_roots_and_relations(db: &mut TestDb<SharedApiSchema>) {
                let SharedApiSchema { authors, posts, .. } = schema;
                db.insert(authors).value(author(ALICE, "Alice")).execute();
                db.insert(authors)
                    .value(author(BOB, "Bob").with_invited_by(ALICE))
                    .execute();
                db.insert(posts)
                    .values([
                        post(10, "Post 1", true, ALICE),
                        post(11, "Post 2", true, ALICE),
                    ])
                    .execute();

                let whitelisted = db
                    .query(authors)
                    .columns(authors.columns().id().name())
                    .order_by(asc(authors.id))
                    .find_many();
                assert_eq!(whitelisted[0].id, Some(ALICE));
                assert_eq!(whitelisted[0].name.as_deref(), Some("Alice"));
                assert!(whitelisted[1].invited_by.is_none());

                let blacklisted = db
                    .query(authors)
                    .omit(authors.columns().invited_by())
                    .r#where(eq(authors.id, BOB))
                    .find_first()
                    .unwrap();
                assert_eq!(blacklisted.name.as_deref(), Some("Bob"));
                assert!(blacklisted.invited_by.is_none());

                let partial_root = db
                    .query(authors)
                    .columns(authors.columns().name())
                    .with(authors.posts().order_by(asc(posts.id)))
                    .r#where(eq(authors.id, ALICE))
                    .find_first()
                    .unwrap();
                assert!(partial_root.id.is_none());
                assert_eq!(partial_root.posts.len(), 2);
                assert_eq!(partial_root.posts[0].title, "Post 1");

                let partial_relation = db
                    .query(authors)
                    .r#where(eq(authors.id, ALICE))
                    .with(
                        authors
                            .posts()
                            .columns(posts.columns().id().title())
                            .order_by(asc(posts.id)),
                    )
                    .find_first()
                    .unwrap();
                assert_eq!(partial_relation.name, "Alice");
                assert_eq!(partial_relation.posts[0].id, Some(10));
                assert_eq!(partial_relation.posts[0].title.as_deref(), Some("Post 1"));
                assert!(partial_relation.posts[0].author_id.is_none());
            }

            /// The generated `{Table}With{Relation}` aliases name nested row types.
            fn count_posts(user: &SharedApiAuthorWithPosts) -> usize {
                user.posts.len()
            }

            fn inviter_name(
                user: &SharedApiAuthorWithInvitedBy<SharedApiAuthorWithPosts>,
            ) -> Option<&str> {
                user.invited_by
                    .as_ref()
                    .map(|inviter| inviter.name.as_str())
            }

            #[drizzle::test($dialect)]
            fn generated_row_aliases_name_nested_results(db: &mut TestDb<SharedApiSchema>) {
                let SharedApiSchema { authors, posts, .. } = schema;
                db.insert(authors).value(author(ALICE, "Alice")).execute();
                db.insert(authors)
                    .value(author(BOB, "Bob").with_invited_by(ALICE))
                    .execute();
                db.insert(posts)
                    .values([
                        post(10, "Post 1", true, ALICE),
                        post(11, "Post 2", true, ALICE),
                        post(12, "Bob Post", true, BOB),
                    ])
                    .execute();

                let users: Vec<SharedApiAuthorWithPosts> = db
                    .query(authors)
                    .with(authors.posts())
                    .order_by(asc(authors.id))
                    .find_many();
                assert_eq!(count_posts(&users[0]), 2);
                assert_eq!(count_posts(&users[1]), 1);

                let users: Vec<SharedApiAuthorWithInvitedBy<SharedApiAuthorWithPosts>> = db
                    .query(authors)
                    .with(authors.posts())
                    .with(authors.invited_by())
                    .order_by(asc(authors.id))
                    .find_many();
                assert_eq!(inviter_name(&users[0]), None);
                assert_eq!(inviter_name(&users[1]), Some("Alice"));
            }

            #[drizzle::test($dialect)]
            fn prepared_relational_queries_bind_placeholders(db: &mut TestDb<SharedApiSchema>) {
                let SharedApiSchema { authors, posts, .. } = schema;
                db.insert(authors)
                    .values([author(ALICE, "Alice"), author(BOB, "Bob")])
                    .execute();
                db.insert(posts)
                    .values([
                        post(10, "AAA", true, ALICE),
                        post(11, "BBB", true, ALICE),
                        post(12, "CCC", true, ALICE),
                        post(13, "Bob Post", true, BOB),
                    ])
                    .execute();

                let name = authors.name.placeholder("shared_api_name");
                let root_limit = drizzle::core::Placeholder::typed::<$integer>("shared_api_root");
                let post_limit = drizzle::core::Placeholder::typed::<$integer>("shared_api_posts");
                let prepared = db
                    .query(authors)
                    .with(authors.posts().order_by(asc(posts.title)).limit(post_limit))
                    .r#where(eq(authors.name, name))
                    .order_by(asc(authors.name))
                    .limit(root_limit)
                    .prepare();

                let users = prepared.find_many(
                    drizzle_client!(),
                    [name.bind("Alice"), root_limit.bind(1), post_limit.bind(2)],
                );
                assert_eq!(users.len(), 1);
                assert_eq!(
                    users[0]
                        .posts
                        .iter()
                        .map(|post| post.title.as_str())
                        .collect::<Vec<_>>(),
                    ["AAA", "BBB"]
                );
            }

            // ---------------------------------------------------------- transactions

            #[drizzle::test($dialect)]
            fn transactions_see_their_own_writes(db: &mut TestDb<SharedApiSchema>) {
                let SharedApiSchema { authors, posts, .. } = schema;
                db.insert(authors)
                    .values([author(ALICE, "Alice"), author(BOB, "Bob")])
                    .execute();
                db.insert(posts)
                    .values([post(10, "A1", true, ALICE), post(11, "A2", true, ALICE)])
                    .execute();

                let rows = result!(db.transaction($transaction_config, |tx| {
                    result!(
                        tx.insert(posts)
                            .value(post(12, "A3", true, ALICE))
                            .execute()
                    )?;
                    result!(
                        tx.query(authors)
                            .r#where(eq(authors.id, ALICE))
                            .with(authors.posts())
                            .find_many()
                    )
                }))
                .expect("transaction commits");
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].posts.len(), 3);

                let committed = db
                    .query(authors)
                    .r#where(eq(authors.id, ALICE))
                    .with(authors.posts())
                    .find_first()
                    .unwrap();
                assert_eq!(committed.posts.len(), 3);

                let found = result!(db.transaction($transaction_config, |tx| {
                    result!(
                        tx.query(authors)
                            .columns(authors.columns().id().name())
                            .r#where(eq(authors.name, "Bob"))
                            .find_first()
                    )
                }))
                .expect("transaction commits");
                let bob = found.expect("Bob is found");
                assert_eq!(bob.name.as_deref(), Some("Bob"));
                assert_eq!(bob.id, Some(BOB));
            }

            #[drizzle::test($dialect)]
            fn rolled_back_writes_disappear_from_relational_queries(
                db: &mut TestDb<SharedApiSchema>,
            ) {
                let SharedApiSchema { authors, posts, .. } = schema;
                db.insert(authors).value(author(ALICE, "Alice")).execute();

                let rolled_back: drizzle::Result<()> =
                    result!(db.transaction($transaction_config, |tx| {
                        result!(
                            tx.insert(posts)
                                .value(post(10, "Uncommitted", true, ALICE))
                                .execute()
                        )?;
                        let rows = result!(tx.query(authors).with(authors.posts()).find_many())?;
                        assert_eq!(rows[0].posts.len(), 1, "tx.query sees uncommitted rows");
                        Err(drizzle::error::DrizzleError::Other("roll back".into()))
                    }));
                assert!(rolled_back.is_err());

                let rows = db.query(authors).with(authors.posts()).find_many();
                assert!(rows[0].posts.is_empty(), "rollback discards the insert");
            }
        }
    };
}

pub(crate) use shared_relational_api_suite;
