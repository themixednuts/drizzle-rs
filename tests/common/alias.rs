/// Portable table-alias behavior: `Table::alias::<Tag>()` in FROM/WHERE, self
/// joins, joins between two aliased tables, and coexistence with the unaliased
/// table. Aliased column names still decode into `FromRow` structs.
macro_rules! shared_alias_suite {
    ($dialect:ident, $table:ident, $schema:ident, $from_row:ident) => {
        mod shared_alias {
            use super::*;
            use crate::common::helpers::sql_shape;
            use drizzle::core::expr::{alias, and, eq, gt, neq};
            use drizzle_core::tag;

            #[$table(NAME = "shared_alias_people")]
            struct SharedAliasPerson {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
                email: Option<String>,
            }

            #[$table(NAME = "shared_alias_posts")]
            struct SharedAliasPost {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                title: String,
                #[column(references = SharedAliasPerson::id)]
                author_id: i32,
                published: bool,
            }

            #[derive($schema)]
            struct SharedAliasSchema {
                people: SharedAliasPerson,
                posts: SharedAliasPost,
            }

            #[derive(Debug, $from_row)]
            struct PersonRow {
                id: i32,
                name: String,
            }

            #[derive(Debug, $from_row)]
            struct NamePair {
                first: String,
                second: String,
            }

            #[derive(Debug, $from_row)]
            struct AuthoredPost {
                author_name: String,
                post_title: String,
            }

            tag!(PeopleTag, "p");
            tag!(LeftTag, "lhs");
            tag!(RightTag, "rhs");
            tag!(AuthorTag, "author");
            tag!(PostTag, "post");

            type PersonSeed = InsertSharedAliasPerson<
                'static,
                (
                    SharedAliasPersonIdSet,
                    SharedAliasPersonNameSet,
                    SharedAliasPersonEmailSet,
                ),
            >;

            fn people_seed() -> [PersonSeed; 3] {
                [
                    InsertSharedAliasPerson::new("alice")
                        .with_id(1)
                        .with_email("shared@example.com"),
                    InsertSharedAliasPerson::new("bob")
                        .with_id(2)
                        .with_email("shared@example.com"),
                    InsertSharedAliasPerson::new("charlie")
                        .with_id(3)
                        .with_email("solo@example.com"),
                ]
            }

            #[drizzle::test($dialect)]
            fn aliased_table_renders_and_filters_through_the_alias(
                db: &mut TestDb<SharedAliasSchema>,
            ) {
                let SharedAliasSchema { people, .. } = schema;
                db.insert(people).values(people_seed()).execute();

                let p = SharedAliasPerson::alias::<PeopleTag>();
                assert_eq!(p.name(), "p");

                let stmt = db.select((p.id, p.name)).from(p).r#where(eq(p.name, "bob"));
                let shape = sql_shape(&stmt.to_sql().sql());
                assert!(
                    shape.contains("FROMshared_alias_peopleASp")
                        || shape.contains("FROMshared_alias_peoplep"),
                    "alias should be applied in FROM: {shape}"
                );
                assert!(
                    shape.contains("WHEREp.name=?"),
                    "predicates should qualify through the alias: {shape}"
                );

                let rows: Vec<PersonRow> = stmt.all();
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].id, 2);
                assert_eq!(rows[0].name, "bob");
            }

            #[drizzle::test($dialect)]
            fn aliased_columns_compose_in_compound_predicates(db: &mut TestDb<SharedAliasSchema>) {
                let SharedAliasSchema { people, .. } = schema;
                db.insert(people).values(people_seed()).execute();

                let p = SharedAliasPerson::alias::<PeopleTag>();
                let rows: Vec<PersonRow> = db
                    .select((p.id, p.name))
                    .from(p)
                    .r#where(and(gt(p.id, 1), neq(p.name, "charlie")))
                    .all();
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].name, "bob");
            }

            #[drizzle::test($dialect)]
            fn self_join_through_two_aliases(db: &mut TestDb<SharedAliasSchema>) {
                let SharedAliasSchema { people, .. } = schema;
                db.insert(people).values(people_seed()).execute();

                let lhs = SharedAliasPerson::alias::<LeftTag>();
                let rhs = SharedAliasPerson::alias::<RightTag>();

                let mut pairs: Vec<NamePair> = db
                    .select((alias(lhs.name, "first"), alias(rhs.name, "second")))
                    .from(lhs)
                    .inner_join((rhs, eq(lhs.email, rhs.email)))
                    .r#where(neq(lhs.id, rhs.id))
                    .all();
                pairs.sort_by(|a, b| (&a.first, &a.second).cmp(&(&b.first, &b.second)));

                let pairs = pairs
                    .iter()
                    .map(|pair| (pair.first.as_str(), pair.second.as_str()))
                    .collect::<Vec<_>>();
                assert_eq!(pairs, [("alice", "bob"), ("bob", "alice")]);
            }

            #[drizzle::test($dialect)]
            fn joins_between_two_aliased_tables(db: &mut TestDb<SharedAliasSchema>) {
                let SharedAliasSchema { people, posts } = schema;
                db.insert(people).values(people_seed()).execute();
                db.insert(posts)
                    .values([
                        InsertSharedAliasPost::new("First Post", 1, true).with_id(1),
                        InsertSharedAliasPost::new("Second Post", 2, true).with_id(2),
                        InsertSharedAliasPost::new("Draft", 1, false).with_id(3),
                    ])
                    .execute();

                let author = SharedAliasPerson::alias::<AuthorTag>();
                let post = SharedAliasPost::alias::<PostTag>();

                let rows: Vec<AuthoredPost> = db
                    .select((
                        alias(author.name, "author_name"),
                        alias(post.title, "post_title"),
                    ))
                    .from(author)
                    .inner_join((post, eq(author.id, post.author_id)))
                    .r#where(eq(post.published, true))
                    .order_by([asc(author.name)])
                    .all();

                let rows = rows
                    .iter()
                    .map(|row| (row.author_name.as_str(), row.post_title.as_str()))
                    .collect::<Vec<_>>();
                assert_eq!(rows, [("alice", "First Post"), ("bob", "Second Post")]);
            }

            #[drizzle::test($dialect)]
            fn alias_and_base_table_coexist_in_one_query(db: &mut TestDb<SharedAliasSchema>) {
                let SharedAliasSchema { people, .. } = schema;
                db.insert(people).values(people_seed()).execute();

                // Join the base table to an alias of itself: the alias must not
                // rewrite references to the unaliased table.
                let other = SharedAliasPerson::alias::<PeopleTag>();
                let stmt = db
                    .select((alias(people.name, "first"), alias(other.name, "second")))
                    .from(people)
                    .inner_join((other, eq(people.email, other.email)))
                    .r#where(and(eq(people.name, "alice"), neq(other.id, people.id)));
                let shape = sql_shape(&stmt.to_sql().sql());
                assert!(
                    shape.contains("shared_alias_people.email=p.email"),
                    "base and aliased references must both survive: {shape}"
                );

                let rows: Vec<NamePair> = stmt.all();
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].first, "alice");
                assert_eq!(rows[0].second, "bob");
            }
        }
    };
}

pub(crate) use shared_alias_suite;
