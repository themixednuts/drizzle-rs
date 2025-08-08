use drizzle_rs::prelude::*;

#[test]
fn test_alias_functionality() {
    // Test basic alias
    let table_alias: SQL<String> = SQL::raw("users").alias("u");
    assert_eq!(table_alias.sql(), "users AS u");

    #[allow(unused_doc_comments)]
    ///```
    /// let user = User::default();
    /// let parent = User::Alias::new("parent");
    /// db.select().from(parent).join(Join::Join, eq(parent(User::id), ));
    ///
    /// ```
    let column_alias: SQL<String> = SQL::raw("user_name").alias("name");
    assert_eq!(column_alias.sql(), "user_name AS name");
}

#[test]
fn test_subquery_functionality() {
    // Test basic subquery
    let subquery: SQL<String> = SQL::raw("SELECT id FROM posts WHERE published = true").subquery();
    assert_eq!(
        subquery.sql(),
        "(SELECT id FROM posts WHERE published = true)"
    );
}

#[test]
fn test_nested_alias_subquery() {
    // Test complex nested structures
    let complex: SQL<String> = SQL::raw("SELECT * FROM")
        .append(SQL::raw("users").alias("u"))
        .append_raw(" WHERE u.id IN ")
        .append(SQL::raw("SELECT user_id FROM posts WHERE published = true").subquery());

    let expected =
        "SELECT * FROM users AS u WHERE u.id IN (SELECT user_id FROM posts WHERE published = true)";
    assert_eq!(complex.sql(), expected);
}

#[test]
fn test_aliased_subquery() {
    // Test subquery with alias
    let aliased_subquery: SQL<String> = SQL::raw("SELECT COUNT(*) FROM posts")
        .subquery()
        .alias("post_count");
    assert_eq!(
        aliased_subquery.sql(),
        "(SELECT COUNT(*) FROM posts) AS post_count"
    );
}
