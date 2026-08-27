/// Cross-dialect CRUD and join behavior.
///
/// Each dialect invokes this once after importing its prelude. Mutation results
/// are intentionally ignored because drivers expose different result metadata;
/// subsequent reads verify the persisted state instead.
macro_rules! shared_crud_join_suite {
    ($dialect:ident, $table:ident, $schema:ident) => {
        mod shared_crud_join_contract {
            use super::*;
            use drizzle::core::{asc, expr::eq};

            #[$table(NAME = "shared_crud_users")]
            struct SharedCrudUser {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
                active: bool,
                nickname: Option<String>,
            }

            #[$table(NAME = "shared_crud_posts")]
            struct SharedCrudPost {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                #[column(REFERENCES = SharedCrudUser::id)]
                user_id: i32,
                title: String,
            }

            #[derive($schema)]
            struct SharedCrudJoinSchema {
                users: SharedCrudUser,
                posts: SharedCrudPost,
            }

            #[drizzle::test($dialect)]
            fn insert_select_update_and_delete(db: &mut TestDb<SharedCrudJoinSchema>) {
                let SharedCrudJoinSchema { users, posts } = schema;

                db.insert(users)
                    .value(InsertSharedCrudUser::new("Alice", true).with_id(1))
                    .execute();
                db.insert(users)
                    .value(
                        InsertSharedCrudUser::new("Bob", false)
                            .with_id(2)
                            .with_nickname("Bobby"),
                    )
                    .execute();
                db.insert(posts)
                    .values([
                        InsertSharedCrudPost::new(1, "First").with_id(10),
                        InsertSharedCrudPost::new(1, "Second").with_id(11),
                    ])
                    .execute();

                let alice: SelectSharedCrudUser =
                    db.select(()).from(users).r#where(eq(users.id, 1)).get();
                assert_eq!(alice.name, "Alice");
                assert!(alice.active);
                assert_eq!(alice.nickname, None);

                db.update(users)
                    .set(
                        UpdateSharedCrudUser::default()
                            .with_name("Robert")
                            .with_active(true),
                    )
                    .r#where(eq(users.id, 2))
                    .execute();

                let robert: SelectSharedCrudUser =
                    db.select(()).from(users).r#where(eq(users.id, 2)).get();
                assert_eq!(robert.name, "Robert");
                assert!(robert.active);
                assert_eq!(robert.nickname.as_deref(), Some("Bobby"));

                db.delete(posts)
                    .r#where(eq(posts.title, "Second"))
                    .execute();

                let remaining_posts: Vec<SelectSharedCrudPost> = db.select(()).from(posts).all();
                assert_eq!(remaining_posts.len(), 1);
                assert_eq!(remaining_posts[0].title, "First");
            }

            #[drizzle::test($dialect)]
            fn inner_and_left_joins_decode_joined_rows(db: &mut TestDb<SharedCrudJoinSchema>) {
                let SharedCrudJoinSchema { users, posts } = schema;

                db.insert(users)
                    .values([
                        InsertSharedCrudUser::new("Alice", true).with_id(1),
                        InsertSharedCrudUser::new("Bob", true).with_id(2),
                    ])
                    .execute();
                db.insert(posts)
                    .value(InsertSharedCrudPost::new(1, "Alice's post").with_id(10))
                    .execute();

                let inner: Vec<(SelectSharedCrudUser, SelectSharedCrudPost)> = db
                    .select(())
                    .from(users)
                    .inner_join((posts, eq(posts.user_id, users.id)))
                    .all();
                assert_eq!(inner.len(), 1);
                assert_eq!(inner[0].0.name, "Alice");
                assert_eq!(inner[0].1.title, "Alice's post");

                let left: Vec<(SelectSharedCrudUser, Option<SelectSharedCrudPost>)> = db
                    .select(())
                    .from(users)
                    .left_join((posts, eq(posts.user_id, users.id)))
                    .order_by(asc(users.id))
                    .all();
                assert_eq!(left.len(), 2);
                assert_eq!(left[0].0.name, "Alice");
                assert_eq!(
                    left[0].1.as_ref().map(|post| post.title.as_str()),
                    Some("Alice's post")
                );
                assert_eq!(left[1].0.name, "Bob");
                assert!(left[1].1.is_none());
            }

            #[drizzle::test($dialect)]
            fn set_operations_compose_dialect_queries(db: &mut TestDb<SharedCrudJoinSchema>) {
                let SharedCrudJoinSchema { users, .. } = schema;

                db.insert(users)
                    .values([
                        InsertSharedCrudUser::new("Alice", true).with_id(1),
                        InsertSharedCrudUser::new("Bob", true).with_id(2),
                    ])
                    .execute();

                let bob = drizzle::$dialect::builder::QueryBuilder::new::<SharedCrudJoinSchema>()
                    .select(())
                    .from(users)
                    .r#where(eq(users.name, "Bob"));
                let selected: Vec<SelectSharedCrudUser> =
                    db.select(()).from(users).union_all(bob).all();
                assert_eq!(selected.len(), 3);
                assert_eq!(selected.iter().filter(|row| row.name == "Bob").count(), 2);

                let bob = drizzle::$dialect::builder::QueryBuilder::new::<SharedCrudJoinSchema>()
                    .select(())
                    .from(users)
                    .r#where(eq(users.name, "Bob"));
                let union: Vec<SelectSharedCrudUser> = db.select(()).from(users).union(bob).all();
                assert_eq!(union.len(), 2);

                let bob = drizzle::$dialect::builder::QueryBuilder::new::<SharedCrudJoinSchema>()
                    .select(())
                    .from(users)
                    .r#where(eq(users.name, "Bob"));
                let intersection: Vec<SelectSharedCrudUser> =
                    db.select(()).from(users).intersect(bob).all();
                assert_eq!(intersection.len(), 1);
                assert_eq!(intersection[0].name, "Bob");

                let bob = drizzle::$dialect::builder::QueryBuilder::new::<SharedCrudJoinSchema>()
                    .select(())
                    .from(users)
                    .r#where(eq(users.name, "Bob"));
                let difference: Vec<SelectSharedCrudUser> =
                    db.select(()).from(users).except(bob).all();
                assert_eq!(difference.len(), 1);
                assert_eq!(difference[0].name, "Alice");
            }
        }
    };
}

pub(crate) use shared_crud_join_suite;
