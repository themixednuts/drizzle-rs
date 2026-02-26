#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
use drizzle::core::expr::*;
use drizzle::{sql, sqlite::prelude::*};
use drizzle_macros::sqlite_test;
use drizzle_sqlite::values::SQLiteValue;

#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::ComplexSchema;
use crate::common::schema::sqlite::{InsertSimple, SelectSimple, Simple, SimpleSchema};

sqlite_test!(test_simple_select_all_sql_generation, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let sql = db.select(()).from(simple).to_sql();
    assert_eq!(
        sql.sql(),
        r#"SELECT "simple"."id", "simple"."name" FROM "simple""#
    );

    // Also verify via DB execution
    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("alice").with_id(1), InsertSimple::new("bob").with_id(2)])
            => execute
    );
    let results: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple) => all);
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].name, "alice");
    assert_eq!(results[1].name, "bob");
});

#[cfg(feature = "uuid")]
sqlite_test!(test_complex_select_all_sql_generation, ComplexSchema, {
    let ComplexSchema { complex } = schema;

    let sql_string = db.select(()).from(complex).to_sql().sql();

    #[cfg(not(feature = "serde"))]
    assert_eq!(
        sql_string,
        r#"SELECT "complex"."id", "complex"."name", "complex"."email", "complex"."age", "complex"."score", "complex"."active", "complex"."role", "complex"."description", "complex"."data_blob", "complex"."created_at" FROM "complex""#
    );

    #[cfg(feature = "serde")]
    assert_eq!(
        sql_string,
        r#"SELECT "complex"."id", "complex"."name", "complex"."email", "complex"."age", "complex"."score", "complex"."active", "complex"."role", "complex"."description", "complex"."metadata", "complex"."config", "complex"."data_blob", "complex"."created_at" FROM "complex""#
    );
});

sqlite_test!(test_select_all_with_where_clause, SimpleSchema, {
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
    drizzle_exec!(
        db.insert(simple)
            .values([
                InsertSimple::new("test").with_id(1),
                InsertSimple::new("other").with_id(2),
            ])
            => execute
    );
    let results: Vec<SelectSimple> = drizzle_exec!(
        db.select(()).from(simple).r#where(eq(Simple::name, "test")) => all
    );
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "test");
});

sqlite_test!(test_select_specific_columns_vs_select_all, SimpleSchema, {
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
    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("alice").with_id(1)])
            => execute
    );
    let all_results: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple) => all);
    let specific_results: Vec<SelectSimple> = drizzle_exec!(
        db.select((simple.id, simple.name)).from(simple) => all
    );
    assert_eq!(all_results.len(), specific_results.len());
    assert_eq!(all_results[0].id, specific_results[0].id);
    assert_eq!(all_results[0].name, specific_results[0].name);
});

sqlite_test!(test_sql_macro, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let id = 4;
    drizzle_try!(
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

    let results: Vec<SelectSimple> = drizzle_try!(db.all(query))?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, id);
    assert_eq!(results[0].name, "test");
});

sqlite_test!(test_sql_printf_style, SimpleSchema, {
    let SimpleSchema { simple } = schema;
    let id = 5;
    let name = "printf_test";

    drizzle_try!(
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
});

sqlite_test!(test_sql_mixed_named_positional, SimpleSchema, {
    let SimpleSchema { simple } = schema;
    let id = 6;
    let name = "mixed_test";

    drizzle_try!(
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
});

sqlite_test!(
    test_with_subquery_parenthesized_in_comparison,
    SimpleSchema,
    {
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
            sql.contains(r#""simple"."id" >(WITH filtered_ids AS"#),
            "sql: {sql}"
        );
    }
);

sqlite_test!(
    test_with_subquery_parenthesized_in_set_and_funcs,
    SimpleSchema,
    {
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
            in_sql.contains(r#""simple"."id" IN (WITH filtered_ids AS"#),
            "sql: {in_sql}"
        );

        let with_subquery = builder
            .with(&filtered_ids)
            .select(filtered_ids.id)
            .from(&filtered_ids);
        let func_sql = db.select(avg(with_subquery)).from(simple).to_sql().sql();
        assert!(
            func_sql.contains(r#"AVG ((WITH filtered_ids AS"#),
            "sql: {func_sql}"
        );
    }
);
