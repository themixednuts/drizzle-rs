/// Cross-dialect relational behavior suite.
///
/// Each dialect invokes this once with its table and schema macros. The test
/// body is deliberately shared; dialect folders contain only the invocation
/// and any SQL-shape tests unique to that dialect.
macro_rules! shared_relational_query_suite {
    ($dialect:ident, $table:ident, $schema:ident, $integer:path, $transaction_config:expr) => {
        mod shared_relational_query {
            use super::*;

            #[$table(NAME = "shared_query_users")]
            struct SharedQueryUser {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
                #[column(REFERENCES = SharedQueryUser::id, RELATION = "reports")]
                manager_id: Option<i32>,
            }

            #[$table(NAME = "shared_query_posts")]
            struct SharedQueryPost {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                #[column(REFERENCES = SharedQueryUser::id, RELATION = "posts")]
                author_id: i32,
                title: String,
                rank: i32,
            }

            #[$table(NAME = "shared_query_tags")]
            struct SharedQueryTag {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
            }

            #[$table(NAME = "shared_query_post_tags")]
            struct SharedQueryPostTag {
                #[column(REFERENCES = SharedQueryPost::id)]
                post_id: i32,
                #[column(REFERENCES = SharedQueryTag::id)]
                tag_id: i32,
            }

            #[derive($schema)]
            struct SharedRelationalSchema {
                users: SharedQueryUser,
                posts: SharedQueryPost,
                tags: SharedQueryTag,
                post_tags: SharedQueryPostTag,
            }

            #[drizzle::test($dialect)]
            fn shared_relational_behavior(db: &mut TestDb<SharedRelationalSchema>) {
                let SharedRelationalSchema {
                    users,
                    posts,
                    tags,
                    post_tags,
                } = schema;

                db.insert(users)
                    .values([
                        InsertSharedQueryUser::new("Alice").with_id(1),
                        InsertSharedQueryUser::new("Carol").with_id(3),
                    ])
                    .execute();
                db.insert(users)
                    .value(
                        InsertSharedQueryUser::new("Bob")
                            .with_id(2)
                            .with_manager_id(1),
                    )
                    .execute();
                db.insert(posts)
                    .values([
                        InsertSharedQueryPost::new(1, "A-1", 2).with_id(10),
                        InsertSharedQueryPost::new(1, "A-2", 1).with_id(11),
                        InsertSharedQueryPost::new(2, "B-1", 1).with_id(12),
                    ])
                    .execute();
                db.insert(tags)
                    .values([
                        InsertSharedQueryTag::new("rust").with_id(20),
                        InsertSharedQueryTag::new("sql").with_id(21),
                    ])
                    .execute();
                db.insert(post_tags)
                    .values([
                        InsertSharedQueryPostTag::new(10, 20),
                        InsertSharedQueryPostTag::new(10, 21),
                    ])
                    .execute();

                let filtered = db
                    .query(users)
                    .r#where(eq(users.name, "Alice"))
                    .with(
                        users
                            .posts()
                            .r#where(eq(posts.rank, 1))
                            .order_by(asc(posts.rank))
                            .limit(1)
                            .with(posts.shared_query_tags()),
                    )
                    .find_many();
                assert_eq!(filtered.len(), 1);
                assert_eq!(filtered[0].posts.len(), 1);
                assert_eq!(filtered[0].posts[0].title, "A-2");
                assert!(filtered[0].posts[0].shared_query_tags.is_empty());

                let posts_with_authors = db
                    .query(posts)
                    .order_by(asc(posts.id))
                    .with(posts.author())
                    .find_many();
                assert_eq!(posts_with_authors.len(), 3);
                assert_eq!(posts_with_authors[0].author.name, "Alice");

                let nested_one = db
                    .query(posts)
                    .r#where(eq(posts.id, 12))
                    .with(posts.author().with(users.manager()))
                    .find_first()
                    .unwrap();
                assert_eq!(nested_one.author.name, "Bob");
                assert_eq!(
                    nested_one
                        .author
                        .manager
                        .as_ref()
                        .map(|manager| manager.name.as_str()),
                    Some("Alice")
                );

                let reports = db
                    .query(users)
                    .r#where(eq(users.id, 1))
                    .with(users.reports().order_by(asc(users.id)))
                    .find_first()
                    .unwrap();
                assert_eq!(reports.reports.len(), 1);
                assert_eq!(reports.reports[0].name, "Bob");

                let no_manager = db
                    .query(users)
                    .r#where(eq(users.id, 3))
                    .with(users.manager())
                    .find_first()
                    .unwrap();
                assert!(no_manager.manager.is_none());
                let missing = db.query(users).r#where(eq(users.id, 999)).find_first();
                assert!(missing.is_none());

                let empty = db
                    .query(users)
                    .r#where(eq(users.id, 3))
                    .with(users.posts())
                    .find_first()
                    .unwrap();
                assert!(empty.posts.is_empty());

                let tagged = db
                    .query(posts)
                    .r#where(eq(posts.id, 10))
                    .with(posts.shared_query_tags().order_by(asc(tags.id)))
                    .find_first()
                    .unwrap();
                assert_eq!(tagged.shared_query_tags.len(), 2);

                let partial = db
                    .query(users)
                    .columns(users.columns().name())
                    .order_by(asc(users.id))
                    .find_many();
                assert_eq!(partial.len(), 3);
                assert_eq!(partial[0].name.as_deref(), Some("Alice"));
                assert!(partial[0].id.is_none());

                let name = users.name.placeholder("shared_name");
                let root_limit = drizzle::core::Placeholder::typed::<$integer>("shared_root_limit");
                let post_limit = drizzle::core::Placeholder::typed::<$integer>("shared_post_limit");
                let prepared = db
                    .query(users)
                    .r#where(eq(users.name, name))
                    .with(users.posts().order_by(asc(posts.id)).limit(post_limit))
                    .limit(root_limit)
                    .prepare();
                let prepared_rows = prepared.find_many(
                    drizzle_client!(),
                    [name.bind("Alice"), root_limit.bind(1), post_limit.bind(1)],
                );
                assert_eq!(prepared_rows.len(), 1);
                assert_eq!(prepared_rows[0].posts.len(), 1);
            }

            #[drizzle::test($dialect)]
            fn relational_queries_share_transaction_state_and_obey_rollback(
                db: &mut TestDb<SharedRelationalSchema>,
            ) {
                let SharedRelationalSchema { users, posts, .. } = schema;
                let rolled_back: drizzle::Result<()> =
                    result!(db.transaction($transaction_config, |tx| {
                        result!(
                            tx.insert(users)
                                .value(InsertSharedQueryUser::new("transactional").with_id(1))
                                .execute()
                        )?;
                        result!(
                            tx.insert(posts)
                                .value(
                                    InsertSharedQueryPost::new(
                                        1,
                                        "visible inside transaction",
                                        1,
                                    )
                                    .with_id(10),
                                )
                                .execute()
                        )?;

                        let user = result!(
                            tx.query(users)
                                .r#where(eq(users.id, 1))
                                .with(users.posts())
                                .find_first()
                        )?
                        .expect("inserted user is visible to its transaction");
                        assert_eq!(user.posts.len(), 1);
                        assert_eq!(user.posts[0].title, "visible inside transaction");

                        Err(drizzle::error::DrizzleError::Other(
                            "rollback relational state".into(),
                        ))
                    }));
                assert!(matches!(
                    rolled_back,
                    Err(drizzle::error::DrizzleError::Other(message))
                        if message == "rollback relational state"
                ));

                let users_after_rollback = db.query(users).with(users.posts()).find_many();
                assert!(users_after_rollback.is_empty());
                let posts_after_rollback: Vec<SelectSharedQueryPost> =
                    db.select(()).from(posts).all();
                assert!(posts_after_rollback.is_empty());
            }
        }
    };
}

pub(crate) use shared_relational_query_suite;
