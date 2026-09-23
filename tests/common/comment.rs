/// The `.comment()` / `.comment_tags()` sqlcommenter helpers.
///
/// Calling `.comment(...)` on any builder prepends an encoded `/* ... */`
/// block to the statement; the block must survive the driver round trip.
macro_rules! shared_comment_suite {
    ($dialect:ident, $table:ident, $schema:ident) => {
        mod shared_comments {
            use super::*;
            use drizzle::core::expr::eq;

            #[$table(NAME = "shared_comment_rows")]
            struct SharedCommentRow {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
            }

            #[derive($schema)]
            struct SharedCommentSchema {
                rows: SharedCommentRow,
            }

            #[drizzle::test($dialect)]
            fn comment_prefixes_every_statement_kind(db: &mut TestDb<SharedCommentSchema>) {
                let SharedCommentSchema { rows } = schema;

                // A single space between the block and the SQL matches upstream
                // drizzle-orm output and the conventional sqlcommenter format.
                let select = db
                    .select(())
                    .from(rows)
                    .comment("trace_id=abc")
                    .to_sql()
                    .sql();
                assert!(select.starts_with("/*trace_id=abc*/ SELECT "), "{select}");

                let filtered = db
                    .select(())
                    .from(rows)
                    .r#where(eq(rows.name, "x"))
                    .comment("slow-query-warn")
                    .to_sql()
                    .sql();
                assert!(
                    filtered.starts_with("/*slow-query-warn*/ SELECT "),
                    "{filtered}"
                );
                assert!(filtered.contains(" WHERE "), "{filtered}");

                let insert = db
                    .insert(rows)
                    .value(InsertSharedCommentRow::new("alice").with_id(1))
                    .comment("ins")
                    .to_sql()
                    .sql();
                assert!(insert.starts_with("/*ins*/ INSERT "), "{insert}");

                let update = db
                    .update(rows)
                    .set(UpdateSharedCommentRow::default().with_name("renamed"))
                    .r#where(eq(rows.id, 1))
                    .comment("upd")
                    .to_sql()
                    .sql();
                assert!(update.starts_with("/*upd*/ UPDATE "), "{update}");

                let delete = db
                    .delete(rows)
                    .r#where(eq(rows.id, 1))
                    .comment("del")
                    .to_sql()
                    .sql();
                assert!(delete.starts_with("/*del*/ DELETE "), "{delete}");
            }

            #[drizzle::test($dialect)]
            fn comment_tags_are_sorted_and_url_encoded(db: &mut TestDb<SharedCommentSchema>) {
                let SharedCommentSchema { rows } = schema;

                let sql = db
                    .select(())
                    .from(rows)
                    .comment_tags([("route", "/users/:id"), ("action", "update")])
                    .to_sql()
                    .sql();
                assert!(
                    sql.starts_with("/*action='update',route='%2Fusers%2F%3Aid'*/ SELECT "),
                    "{sql}"
                );
            }

            #[drizzle::test($dialect)]
            fn comment_cannot_close_its_own_block(db: &mut TestDb<SharedCommentSchema>) {
                let SharedCommentSchema { rows } = schema;

                // Attacker-controlled text containing `/*` or `*/` must not
                // escape the surrounding comment.
                let sql = db
                    .select(())
                    .from(rows)
                    .comment("evil /* escape */ attempt")
                    .to_sql()
                    .sql();
                assert!(
                    sql.starts_with("/*evil / * escape * / attempt*/ SELECT "),
                    "{sql}"
                );
            }

            #[drizzle::test($dialect)]
            fn empty_comments_are_omitted(db: &mut TestDb<SharedCommentSchema>) {
                let SharedCommentSchema { rows } = schema;

                let sql = db.select(()).from(rows).comment("").to_sql().sql();
                assert!(sql.starts_with("SELECT "), "{sql}");

                let sql = db
                    .select(())
                    .from(rows)
                    .comment_tags::<[(&str, &str); 0], _, _>([])
                    .to_sql()
                    .sql();
                assert!(sql.starts_with("SELECT "), "{sql}");
            }

            #[drizzle::test($dialect)]
            fn commented_statements_still_execute(db: &mut TestDb<SharedCommentSchema>) {
                let SharedCommentSchema { rows } = schema;

                db.insert(rows)
                    .values([
                        InsertSharedCommentRow::new("alice").with_id(1),
                        InsertSharedCommentRow::new("bob").with_id(2),
                    ])
                    .comment("seed")
                    .execute();

                let names: Vec<String> = db
                    .select(rows.name)
                    .from(rows)
                    .order_by(asc(rows.id))
                    .comment_tags([("route", "/names")])
                    .all();
                assert_eq!(names, ["alice", "bob"]);

                db.update(rows)
                    .set(UpdateSharedCommentRow::default().with_name("alicia"))
                    .r#where(eq(rows.id, 1))
                    .comment("rename")
                    .execute();
                db.delete(rows)
                    .r#where(eq(rows.id, 2))
                    .comment("prune")
                    .execute();

                let remaining: Vec<String> = db.select(rows.name).from(rows).all();
                assert_eq!(remaining, ["alicia"]);
            }
        }
    };
}

pub(crate) use shared_comment_suite;
