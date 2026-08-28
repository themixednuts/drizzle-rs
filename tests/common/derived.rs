/// Portable derived-table contracts exercised by every SQL dialect.
macro_rules! shared_derived_table_suite {
    ($dialect:ident, $table:ident, $schema:ident) => {
        mod shared_derived_tables {
            use super::*;
            use drizzle::core::expr::{NamedExt as _, count, eq};

            tag!(SharedDerivedCount, "post_count");
            tag!(SharedDerivedNames, "shared_derived_names");
            tag!(SharedDerivedPosts, "shared_derived_posts");
            tag!(
                SharedDerivedProjectedUsers,
                "shared_derived_projected_users"
            );
            tag!(
                SharedDerivedReprojectedUsers,
                "shared_derived_reprojected_users"
            );
            tag!(SharedDerivedStarUsers, "shared_derived_star_users");
            tag!(SharedDerivedUsers, "shared_derived_users");

            #[$table(NAME = "shared_derived_users")]
            struct SharedDerivedUser {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
            }

            #[$table(NAME = "shared_derived_posts")]
            struct SharedDerivedPost {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                user_id: i32,
                title: String,
                rank: i32,
            }

            #[derive($schema)]
            struct SharedDerivedSchema {
                users: SharedDerivedUser,
                posts: SharedDerivedPost,
            }

            #[drizzle::test($dialect)]
            fn derived_tables_preserve_projection_and_row_types(
                db: &mut TestDb<SharedDerivedSchema>,
            ) {
                let SharedDerivedSchema { users, posts } = schema;

                db.insert(users)
                    .values([
                        InsertSharedDerivedUser::new("Ada").with_id(1),
                        InsertSharedDerivedUser::new("Grace").with_id(2),
                    ])
                    .execute();
                db.insert(posts)
                    .values([
                        InsertSharedDerivedPost::new(1, "compiler", 1).with_id(10),
                        InsertSharedDerivedPost::new(1, "notes", 2).with_id(11),
                        InsertSharedDerivedPost::new(2, "database", 1).with_id(12),
                    ])
                    .execute();

                let recent = db
                    .select((posts.user_id, posts.title))
                    .from(posts)
                    .r#where(eq(posts.rank, 1))
                    .alias(SharedDerivedPosts);
                let (recent_user_id, recent_title) = recent.fields();

                let rows: Vec<(String, String)> = db
                    .select((users.name, recent_title))
                    .from(users)
                    .inner_join((recent, eq(users.id, recent_user_id)))
                    .order_by(users.id)
                    .all();
                assert_eq!(
                    rows,
                    vec![
                        ("Ada".to_owned(), "compiler".to_owned()),
                        ("Grace".to_owned(), "database".to_owned()),
                    ]
                );

                let recent = db
                    .select((posts.user_id, posts.title))
                    .from(posts)
                    .r#where(eq(posts.rank, 1))
                    .alias(SharedDerivedPosts);
                let recent_fields = recent.fields();
                let rows: Vec<(i32, String)> =
                    db.select(()).from(recent).order_by(recent_fields.0).all();
                assert_eq!(
                    rows,
                    vec![(1, "compiler".to_owned()), (2, "database".to_owned())]
                );

                let titles = db
                    .select(posts.title)
                    .from(posts)
                    .r#where(eq(posts.rank, 1))
                    .alias(SharedDerivedPosts);
                let title = titles.fields().0;
                let rows: Vec<(String,)> = db.select(()).from(titles).order_by(title).all();
                assert_eq!(
                    rows,
                    vec![("compiler".to_owned(),), ("database".to_owned(),)]
                );

                let all_users = db.select(()).from(users).alias(SharedDerivedUsers);
                let all_user_fields = all_users.fields();
                let rows: Vec<SelectSharedDerivedUser> = db
                    .select(())
                    .from(all_users)
                    .order_by(all_user_fields.id)
                    .all();
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].name, "Ada");
                assert_eq!(rows[1].name, "Grace");

                let projected_users = db
                    .select((users.id, users.name))
                    .from(users)
                    .alias(SharedDerivedProjectedUsers);
                let (projected_id, _) = projected_users.fields();
                let rows: Vec<(i32, String)> = db
                    .select(())
                    .from(projected_users)
                    .order_by(projected_id)
                    .all();
                assert_eq!(rows, vec![(1, "Ada".to_owned()), (2, "Grace".to_owned())]);

                let projected_users = db
                    .select((users.id, users.name))
                    .from(users)
                    .alias(SharedDerivedProjectedUsers);
                let projected_users = db
                    .select(projected_users.fields())
                    .from(projected_users)
                    .alias(SharedDerivedReprojectedUsers);
                let reprojected_id = projected_users.fields().0;
                let rows: Vec<(i32, String)> = db
                    .select(())
                    .from(projected_users)
                    .order_by(reprojected_id)
                    .all();
                assert_eq!(rows, vec![(1, "Ada".to_owned()), (2, "Grace".to_owned())]);

                let projected_users = db
                    .select((users.id, users.name))
                    .from(users)
                    .alias(SharedDerivedProjectedUsers);
                let projected_users = db
                    .select(())
                    .from(projected_users)
                    .alias(SharedDerivedStarUsers);
                let star_id = projected_users.fields().0;
                let rows: Vec<(i32, String)> =
                    db.select(()).from(projected_users).order_by(star_id).all();
                assert_eq!(rows, vec![(1, "Ada".to_owned()), (2, "Grace".to_owned())]);

                let names = db.select(users.name).from(users).alias(SharedDerivedNames);
                let (derived_name,) = names.fields();
                let rows: Vec<(String,)> = db.select(()).from(names).order_by(derived_name).all();
                assert_eq!(rows, vec![("Ada".to_owned(),), ("Grace".to_owned(),)]);

                let names = db.select(users.name).from(users).alias(SharedDerivedNames);
                let derived_name = names.fields().0;
                let rows: Vec<(String, String)> = db
                    .select((users.name, derived_name))
                    .from(users)
                    .cross_join(names)
                    .order_by((users.id, derived_name))
                    .all();
                assert_eq!(
                    rows,
                    vec![
                        ("Ada".to_owned(), "Ada".to_owned()),
                        ("Ada".to_owned(), "Grace".to_owned()),
                        ("Grace".to_owned(), "Ada".to_owned()),
                        ("Grace".to_owned(), "Grace".to_owned()),
                    ]
                );

                let rows: Vec<(String, String)> = db
                    .select((users.name, posts.title))
                    .from(users)
                    .cross_join((posts, eq(users.id, posts.user_id)))
                    .order_by((users.id, posts.id))
                    .all();
                assert_eq!(
                    rows,
                    vec![
                        ("Ada".to_owned(), "compiler".to_owned()),
                        ("Ada".to_owned(), "notes".to_owned()),
                        ("Grace".to_owned(), "database".to_owned()),
                    ]
                );

                let counts = db
                    .select((posts.user_id, count(posts.id).named::<SharedDerivedCount>()))
                    .from(posts)
                    .group_by(posts.user_id)
                    .alias(SharedDerivedPosts);
                let (count_user_id, post_count) = counts.fields();
                let rows: Vec<(String, i64)> = db
                    .select((users.name, post_count))
                    .from(users)
                    .inner_join((counts, eq(users.id, count_user_id)))
                    .order_by(users.id)
                    .all();
                assert_eq!(rows, vec![("Ada".to_owned(), 2), ("Grace".to_owned(), 1)]);
            }
        }
    };
}

/// Derived-table contracts shared by dialects that support LATERAL joins.
#[cfg(any(feature = "mysql", feature = "postgres"))]
macro_rules! shared_lateral_derived_table_suite {
    ($dialect:ident, $table:ident, $schema:ident) => {
        mod shared_lateral_derived_tables {
            use super::*;
            use drizzle::core::expr::eq;

            tag!(SharedLateralPosts, "shared_lateral_posts");

            #[$table(NAME = "shared_lateral_users")]
            struct SharedLateralUser {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
            }

            #[$table(NAME = "shared_lateral_post_rows")]
            struct SharedLateralPost {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                user_id: i32,
                title: String,
            }

            #[derive($schema)]
            struct SharedLateralSchema {
                users: SharedLateralUser,
                posts: SharedLateralPost,
            }

            #[drizzle::test($dialect)]
            fn lateral_joins_preserve_rows_and_left_join_nullability(
                db: &mut TestDb<SharedLateralSchema>,
            ) {
                let SharedLateralSchema { users, posts } = schema;

                db.insert(users)
                    .values([
                        InsertSharedLateralUser::new("Ada").with_id(1),
                        InsertSharedLateralUser::new("Grace").with_id(2),
                    ])
                    .execute();
                db.insert(posts)
                    .values([InsertSharedLateralPost::new(1, "compiler").with_id(10)])
                    .execute();

                let source = db
                    .select(())
                    .from(posts)
                    .r#where(eq(posts.user_id, users.id))
                    .alias(SharedLateralPosts);
                let source_user_id = source.fields().user_id;
                let rows: Vec<(SelectSharedLateralUser, SelectSharedLateralPost)> = db
                    .select(())
                    .from(users)
                    .inner_join_lateral((source, eq(source_user_id, users.id)))
                    .order_by(users.id)
                    .all();
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].0.name, "Ada");
                assert_eq!(rows[0].1.user_id, 1);
                assert_eq!(rows[0].1.title, "compiler");

                let source = db
                    .select(())
                    .from(posts)
                    .r#where(eq(posts.user_id, users.id))
                    .alias(SharedLateralPosts);
                let source_user_id = source.fields().user_id;
                let rows: Vec<(SelectSharedLateralUser, Option<SelectSharedLateralPost>)> = db
                    .select(())
                    .from(users)
                    .left_join_lateral((source, eq(source_user_id, users.id)))
                    .order_by(users.id)
                    .all();
                assert_eq!(rows.len(), 2);
                assert_eq!(
                    rows[0].1.as_ref().map(|post| post.title.as_str()),
                    Some("compiler")
                );
                assert_eq!(rows[1].0.name, "Grace");
                assert!(rows[1].1.is_none());

                let source = db
                    .select(())
                    .from(posts)
                    .r#where(eq(posts.user_id, users.id))
                    .alias(SharedLateralPosts);
                let source_user_id = source.fields().user_id;
                let rows: Vec<(i32,)> = db
                    .select(users.id)
                    .from(users)
                    .left_join_lateral((source, eq(source_user_id, users.id)))
                    .order_by(users.id)
                    .all();
                assert_eq!(rows, vec![(1,), (2,)]);

                let source = db
                    .select(())
                    .from(posts)
                    .r#where(eq(posts.user_id, users.id))
                    .alias(SharedLateralPosts);
                let source_user_id = source.fields().user_id;
                let rows: Vec<(i32,)> = db
                    .select(users.id)
                    .from(users)
                    .inner_join((posts, eq(posts.user_id, users.id)))
                    .left_join_lateral((source, eq(source_user_id, users.id)))
                    .order_by(users.id)
                    .all();
                assert_eq!(rows, vec![(1,)]);

                let source = db
                    .select(())
                    .from(posts)
                    .r#where(eq(posts.user_id, users.id))
                    .alias(SharedLateralPosts);
                let rows: Vec<(SelectSharedLateralUser, SelectSharedLateralPost)> = db
                    .select(())
                    .from(users)
                    .cross_join_lateral(source)
                    .order_by(users.id)
                    .all();
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].0.name, "Ada");
                assert_eq!(rows[0].1.user_id, 1);
                assert_eq!(rows[0].1.title, "compiler");
            }
        }
    };
}

pub(crate) use shared_derived_table_suite;
#[cfg(any(feature = "mysql", feature = "postgres"))]
pub(crate) use shared_lateral_derived_table_suite;
