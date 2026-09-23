#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
use drizzle::core::expr::*;
use drizzle::{sql, sqlite::prelude::*};
use drizzle_sqlite::values::SQLiteValue;

#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::ComplexSchema;
use crate::common::schema::sqlite::{InsertSimple, SelectSimple, Simple, SimpleSchema};

#[drizzle::test]
fn test_simple_select_all_sql_generation(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let sql = db.select(()).from(simple).to_sql();
    assert_eq!(
        sql.sql(),
        r#"SELECT "simple"."id", "simple"."name" FROM "simple""#
    );

    // Also verify via DB execution

    db.insert(simple)
        .values([
            InsertSimple::new("alice").with_id(1),
            InsertSimple::new("bob").with_id(2),
        ])
        .execute();
    let results: Vec<SelectSimple> = db.select(()).from(simple).all();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].name, "alice");
    assert_eq!(results[1].name, "bob");
}

#[cfg(feature = "uuid")]
#[drizzle::test]
fn test_complex_select_all_sql_generation(db: &mut TestDb<ComplexSchema>) {
    let ComplexSchema { complex } = schema;

    let sql_string = db.select(()).from(complex).to_sql().sql();

    #[cfg(not(feature = "serde"))]
    assert_eq!(
        sql_string,
        r#"SELECT "complex"."id", "complex"."name", "complex"."email", "complex"."age", "complex"."score", "complex"."active", "complex"."role", "complex"."description", "complex"."data_blob", "complex"."created_at", "complex"."invited_by" FROM "complex""#
    );

    #[cfg(feature = "serde")]
    assert_eq!(
        sql_string,
        r#"SELECT "complex"."id", "complex"."name", "complex"."email", "complex"."age", "complex"."score", "complex"."active", "complex"."role", "complex"."description", "complex"."metadata", "complex"."config", "complex"."data_blob", "complex"."created_at", "complex"."invited_by" FROM "complex""#
    );
}

#[drizzle::test]
fn test_select_all_with_where_clause(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let sql = db
        .select(())
        .from(simple)
        .r#where(eq(Simple::name, "test"))
        .to_sql();

    assert_eq!(
        sql.sql(),
        r#"SELECT "simple"."id", "simple"."name" FROM "simple" WHERE "simple"."name" = ?"#
    );
    let params: Vec<_> = sql.params().collect();
    assert_eq!(params.len(), 1);
    assert_eq!(params[0], &SQLiteValue::Text("test".into()));

    // Also verify via DB execution

    db.insert(simple)
        .values([
            InsertSimple::new("test").with_id(1),
            InsertSimple::new("other").with_id(2),
        ])
        .execute();
    let results: Vec<SelectSimple> = db
        .select(())
        .from(simple)
        .r#where(eq(Simple::name, "test"))
        .all();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "test");
}

#[drizzle::test]
fn test_select_specific_columns_vs_select_all(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let select_all_sql = db.select(()).from(simple).to_sql().sql();
    let select_specific_sql = db
        .select((simple.id, simple.name))
        .from(simple)
        .to_sql()
        .sql();

    let expected = r#"SELECT "simple"."id", "simple"."name" FROM "simple""#;
    assert_eq!(select_all_sql, expected);
    assert_eq!(select_specific_sql, expected);

    // Also verify both produce identical DB results

    db.insert(simple)
        .values([InsertSimple::new("alice").with_id(1)])
        .execute();
    let all_results: Vec<SelectSimple> = db.select(()).from(simple).all();
    let specific_results: Vec<SelectSimple> =
        db.select((simple.id, simple.name)).from(simple).all();
    assert_eq!(all_results.len(), specific_results.len());
    assert_eq!(all_results[0].id, specific_results[0].id);
    assert_eq!(all_results[0].name, specific_results[0].name);
}

#[drizzle::test]
fn test_sql_macro(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let id = 4;
    result!(
        db.insert(simple)
            .values([InsertSimple::new("test").with_id(id)])
            .execute()
    )?;

    let query = sql!("SELECT * FROM {simple} where {simple.id} = {id}");
    let sql = query.sql();
    let params: Vec<_> = query.params().collect();

    assert_eq!(sql, r#"SELECT * FROM "simple" where "simple"."id" = ?"#);
    assert_eq!(params.len(), 1);
    assert_eq!(params[0], &SQLiteValue::Integer(id as i64));

    let results: Vec<SelectSimple> = result!(db.all(query))?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, id);
    assert_eq!(results[0].name, "test");
}

#[drizzle::test]
fn test_sql_printf_style(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;
    let id = 5;
    let name = "printf_test";

    result!(
        db.insert(simple)
            .values([InsertSimple::new(name).with_id(id)])
            .execute()
    )?;

    // Test printf-style syntax: sql!("template", arg1, arg2, ...)
    let query = sql!("SELECT * FROM {} WHERE {} = {}", simple, simple.id, id);
    let sql = query.sql();
    let params: Vec<_> = query.params().collect();

    assert_eq!(sql, r#"SELECT * FROM "simple" WHERE "simple"."id" = ?"#);
    assert_eq!(params.len(), 1);
    assert_eq!(params[0], &SQLiteValue::Integer(id as i64));
}

#[drizzle::test]
fn test_sql_mixed_named_positional(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;
    let id = 6;
    let name = "mixed_test";

    result!(
        db.insert(simple)
            .values([InsertSimple::new(name).with_id(id)])
            .execute()
    )?;

    // Test mixing positional {} and named {simple.id} expressions
    let query = sql!("SELECT * FROM {} WHERE {simple.id} = {}", simple, id);
    let sql = query.sql();
    let params: Vec<_> = query.params().collect();

    assert_eq!(sql, r#"SELECT * FROM "simple" WHERE "simple"."id" = ?"#);
    assert_eq!(params.len(), 1);
    assert_eq!(params[0], &SQLiteValue::Integer(id as i64));
}

#[drizzle::test]
fn test_with_subquery_parenthesized_in_comparison(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;
    let builder = drizzle::sqlite::builder::QueryBuilder::new::<SimpleSchema>();
    let SimpleSchema {
        simple: subquery_simple,
    } = SimpleSchema::new();

    struct FilteredIdsTag;
    impl drizzle::core::Tag for FilteredIdsTag {
        const NAME: &'static str = "filtered_ids";
    }

    let filtered_ids = builder
        .select(subquery_simple.id)
        .from(subquery_simple)
        .r#where(gt(subquery_simple.id, 10))
        .into_cte::<FilteredIdsTag>();

    let with_subquery = builder
        .with(&filtered_ids)
        .select(filtered_ids.id)
        .from(&filtered_ids);

    let sql = db
        .select(simple.id)
        .from(simple)
        .r#where(gt(simple.id, with_subquery))
        .to_sql()
        .sql();

    assert!(
        sql.contains(r#""simple"."id" >(WITH "filtered_ids" AS"#),
        "sql: {sql}"
    );
}

#[drizzle::test]
fn test_with_subquery_parenthesized_in_set_and_funcs(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;
    let builder = drizzle::sqlite::builder::QueryBuilder::new::<SimpleSchema>();
    let SimpleSchema {
        simple: subquery_simple,
    } = SimpleSchema::new();

    struct FilteredIdsTag;
    impl drizzle::core::Tag for FilteredIdsTag {
        const NAME: &'static str = "filtered_ids";
    }

    let filtered_ids = builder
        .select(subquery_simple.id)
        .from(subquery_simple)
        .r#where(gt(subquery_simple.id, 10))
        .into_cte::<FilteredIdsTag>();

    let with_subquery = builder
        .with(&filtered_ids)
        .select(filtered_ids.id)
        .from(&filtered_ids);
    let in_sql = db
        .select(simple.id)
        .from(simple)
        .r#where(in_subquery(simple.id, with_subquery))
        .to_sql()
        .sql();
    assert!(
        in_sql.contains(r#""simple"."id" IN (WITH "filtered_ids" AS"#),
        "sql: {in_sql}"
    );

    let with_subquery = builder
        .with(&filtered_ids)
        .select(filtered_ids.id)
        .from(&filtered_ids);
    let func_sql = db.select(avg(with_subquery)).from(simple).to_sql().sql();
    assert!(
        func_sql.contains(r#"AVG ((WITH "filtered_ids" AS"#),
        "sql: {func_sql}"
    );
}

#[drizzle::test]
fn set_operation_operands_are_wrapped_only_when_needed(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;
    let qb = drizzle::sqlite::builder::QueryBuilder::new::<SimpleSchema>();

    db.insert(simple)
        .values([
            InsertSimple::new("alice").with_id(1),
            InsertSimple::new("bob").with_id(2),
            InsertSimple::new("carol").with_id(3),
        ])
        .execute();

    // A plain compound and a left-to-right chain render as before.
    let plain = qb
        .select(simple.id)
        .from(simple)
        .union(qb.select(simple.id).from(simple));
    assert_eq!(
        plain.to_sql().sql(),
        r#"SELECT "simple"."id" FROM "simple" UNION SELECT "simple"."id" FROM "simple""#
    );
    let chain = qb
        .select(simple.id)
        .from(simple)
        .union(qb.select(simple.id).from(simple))
        .except(qb.select(simple.id).from(simple));
    assert_eq!(
        chain.to_sql().sql(),
        r#"SELECT "simple"."id" FROM "simple" UNION SELECT "simple"."id" FROM "simple" EXCEPT SELECT "simple"."id" FROM "simple""#
    );

    // SQLite rejects a parenthesized compound operand, so an operand that
    // carries its own ORDER BY / LIMIT becomes a derived table.
    let limited_left = db
        .select(simple.id)
        .from(simple)
        .order_by([asc(simple.id)])
        .limit(1)
        .union(qb.select(simple.id).from(simple).r#where(eq(simple.id, 3)));
    assert_eq!(
        limited_left.to_sql().sql(),
        r#"SELECT * FROM (SELECT "simple"."id" FROM "simple" ORDER BY "simple"."id" ASC LIMIT 1) UNION SELECT "simple"."id" FROM "simple" WHERE "simple"."id" = ?"#
    );
    let mut ids: Vec<i32> = limited_left.all();
    ids.sort_unstable();
    assert_eq!(ids, [1, 3]);

    let limited_right = db
        .select(simple.id)
        .from(simple)
        .r#where(eq(simple.id, 1))
        .union(
            qb.select(simple.id)
                .from(simple)
                .order_by([desc(simple.id)])
                .limit(1),
        );
    assert_eq!(
        limited_right.to_sql().sql(),
        r#"SELECT "simple"."id" FROM "simple" WHERE "simple"."id" = ? UNION SELECT * FROM (SELECT "simple"."id" FROM "simple" ORDER BY "simple"."id" DESC LIMIT 1)"#
    );
    let mut ids: Vec<i32> = limited_right.all();
    ids.sort_unstable();
    assert_eq!(ids, [1, 3]);

    // A compound right operand is grouped: {2} ∪ ({1,2,3} − {2}).
    let nested = db
        .select(simple.id)
        .from(simple)
        .r#where(eq(simple.id, 2))
        .union(
            qb.select(simple.id)
                .from(simple)
                .except(qb.select(simple.id).from(simple).r#where(eq(simple.id, 2))),
        );
    assert_eq!(
        nested.to_sql().sql(),
        r#"SELECT "simple"."id" FROM "simple" WHERE "simple"."id" = ? UNION SELECT * FROM (SELECT "simple"."id" FROM "simple" EXCEPT SELECT "simple"."id" FROM "simple" WHERE "simple"."id" = ?)"#
    );
    let mut ids: Vec<i32> = nested.all();
    ids.sort_unstable();
    assert_eq!(ids, [1, 2, 3]);
}

#[drizzle::test]
fn repeated_named_placeholder_binds_one_value(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    db.insert(simple)
        .values([
            InsertSimple::new("alice").with_id(1),
            InsertSimple::new("bob").with_id(2),
        ])
        .execute();

    let name = simple.name.placeholder("name");
    let query = db
        .select(())
        .from(simple)
        .r#where(or(eq(simple.name, name), eq(simple.name, name)));
    assert_eq!(
        query.to_sql().sql(),
        r#"SELECT "simple"."id", "simple"."name" FROM "simple" WHERE ("simple"."name" = :name OR "simple"."name" = :name)"#
    );

    // Builder path: bind the placeholder on the SQL itself.
    let bound = query
        .to_sql()
        .bind([name.bind::<SQLiteValue<'_>, _>("bob")]);
    let rows: Vec<SelectSimple> = result!(db.all(bound))?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, 2);

    // Prepared path.
    let prepared = query.prepare();
    let rows: Vec<SelectSimple> = prepared.all(drizzle_client!(), [name.bind("alice")]);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, 1);
}
