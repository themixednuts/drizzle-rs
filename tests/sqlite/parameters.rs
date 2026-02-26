#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

#[cfg(feature = "uuid")]
use crate::common::schema::sqlite::{ComplexSchema, InsertComplex, Role, UpdateComplex};
use crate::common::schema::sqlite::{InsertSimple, SelectSimple, SimpleSchema, UpdateSimple};
use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;
use drizzle_macros::sqlite_test;

#[cfg(all(feature = "serde", feature = "uuid"))]
sqlite_test!(test_insert_with_placeholders, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Create a typed placeholder from the column
    let user_name = simple.name.placeholder("user_name");

    // Create insert model with typed placeholder
    let insert_data = InsertSimple::new(user_name);

    // Insert the data (should preserve the placeholder in the SQL)
    let insert_result = db.insert(simple).values([insert_data]);

    // Check that the generated SQL contains the placeholder
    let sql_string = insert_result.to_sql().sql();

    // The SQL should contain the named placeholder
    assert!(
        sql_string.contains(":user_name"),
        "SQL should contain the :user_name placeholder"
    );

    // Test that parameters are correctly preserved
    let sql = insert_result.to_sql();
    let params: Vec<_> = sql.params().collect();
    assert!(
        params.is_empty(),
        "Should have no bound parameters since we used a placeholder"
    );
});

sqlite_test!(
    test_insert_with_placeholders_execute_and_retrieve,
    SimpleSchema,
    {
        let SimpleSchema { simple } = schema;

        // Create a typed placeholder from the column
        let user_name = simple.name.placeholder("user_name");

        // Create insert model with typed placeholder
        let insert_data = InsertSimple::new(user_name);

        // Prepare the insert statement and execute it with bound parameters
        let prepared_insert = db.insert(simple).values([insert_data]).prepare();

        // Execute the prepared insert with bound parameters
        let row_count =
            drizzle_exec!(prepared_insert.execute(db.conn(), [user_name.bind("Alice")]));
        assert_eq!(row_count, 1, "Should have inserted one row");

        // Retrieve the data to verify it was inserted correctly
        let results: Vec<SelectSimple> = drizzle_exec!(
            db.select((simple.id, simple.name))
                .from(simple)
                .r#where(eq(simple.name, "Alice"))
                => all
        );

        assert_eq!(results.len(), 1, "Should have found one result");
        assert_eq!(
            results[0].name, "Alice",
            "Name should match the bound placeholder value"
        );
    }
);

sqlite_test!(
    test_parameter_integration_with_query_builder,
    SimpleSchema,
    {
        #[derive(SQLiteFromRow, Default)]
        struct SimpleResult(String);
        let SimpleSchema { simple } = schema;

        // Insert test data
        let test_data = vec![
            InsertSimple::new("alice"),
            InsertSimple::new("bob"),
            InsertSimple::new("charlie"),
        ];
        drizzle_exec!(db.insert(simple).values(test_data) => execute);

        // Test that normal query builder still works (this uses internal parameter binding)
        let results: Vec<SimpleResult> = drizzle_exec!(
            db.select(simple.name)
                .from(simple)
                .r#where(eq(simple.name, "alice"))
                => all
        );

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "alice");

        // Test multiple parameter conditions using multiple queries
        let alice_results: Vec<SimpleResult> = drizzle_exec!(
            db.select(simple.name)
                .from(simple)
                .r#where(eq(simple.name, "alice"))
                => all
        );

        let bob_results: Vec<SimpleResult> = drizzle_exec!(
            db.select(simple.name)
                .from(simple)
                .r#where(eq(simple.name, "bob"))
                => all
        );

        assert_eq!(alice_results.len(), 1);
        assert_eq!(bob_results.len(), 1);
        assert_eq!(alice_results[0].0, "alice");
        assert_eq!(bob_results[0].0, "bob");
    }
);

sqlite_test!(test_update_with_placeholders_sql, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Create typed placeholders from columns
    let new_name = simple.name.placeholder("new_name");
    let old_name = simple.name.placeholder("old_name");

    // Create update with placeholder in SET and WHERE
    let update = UpdateSimple::default().with_name(new_name);
    let stmt = db
        .update(simple)
        .set(update)
        .r#where(eq(simple.name, old_name));

    let sql = stmt.to_sql();
    let sql_string = sql.sql();

    // Verify SQL structure
    assert!(
        sql_string.starts_with("UPDATE"),
        "Should be an UPDATE statement, got: {}",
        sql_string
    );
    assert!(
        sql_string.contains("\"simple\""),
        "Should reference the simple table, got: {}",
        sql_string
    );
    assert!(
        sql_string.contains(":new_name"),
        "SET clause should contain :new_name placeholder, got: {}",
        sql_string
    );
    assert!(
        sql_string.contains(":old_name"),
        "WHERE clause should contain :old_name placeholder, got: {}",
        sql_string
    );

    // All values are placeholders, so there should be no bound parameters
    let params: Vec<_> = sql.params().collect();
    assert!(
        params.is_empty(),
        "Should have no bound parameters since all values are placeholders, got {} params",
        params.len()
    );
});

sqlite_test!(test_update_with_placeholders_execute, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert initial data
    drizzle_exec!(
        db.insert(simple)
            .values([InsertSimple::new("original_name")])
            => execute
    );

    // Create typed placeholders from columns
    let new_name = simple.name.placeholder("new_name");
    let old_name = simple.name.placeholder("old_name");

    // Create update with placeholders and prepare it
    let update = UpdateSimple::default().with_name(new_name);
    let prepared = db
        .update(simple)
        .set(update)
        .r#where(eq(simple.name, old_name))
        .prepare();

    // Execute with bound parameters
    let update_count = drizzle_exec!(prepared.execute(
        db.conn(),
        [
            new_name.bind("updated_name"),
            old_name.bind("original_name")
        ]
    ));
    drizzle_assert_eq!(1, update_count, "Should have updated one row");

    // Verify the new name exists
    let results: Vec<SelectSimple> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "updated_name"))
            => all
    );
    assert_eq!(results.len(), 1, "Should find the updated row");
    assert_eq!(results[0].name, "updated_name");

    // Verify the original name is gone
    let old_results: Vec<SelectSimple> = drizzle_exec!(
        db.select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "original_name"))
            => all
    );
    assert_eq!(old_results.len(), 0, "Original name should no longer exist");
});

#[cfg(feature = "uuid")]
sqlite_test!(
    test_update_with_mixed_values_and_placeholders,
    ComplexSchema,
    {
        #[allow(dead_code)]
        #[derive(SQLiteFromRow, Debug)]
        struct ComplexResult {
            name: String,
            email: Option<String>,
            age: Option<i32>,
            score: Option<f64>,
        }

        let ComplexSchema { complex } = schema;

        // Insert initial record with known values
        let insert_data = InsertComplex::new("alice", true, Role::User)
            .with_id(uuid::Uuid::new_v4())
            .with_email("alice@old.com".to_string())
            .with_age(25)
            .with_score(90.5);
        drizzle_exec!(db.insert(complex).values([insert_data]) => execute);

        // Create typed placeholder from column
        let new_age = complex.age.placeholder("new_age");

        // Mix concrete value (email) with placeholder (age) in the same update
        let update = UpdateComplex::default()
            .with_email("alice@new.com".to_string())
            .with_age(new_age);

        let prepared = db
            .update(complex)
            .set(update)
            .r#where(eq(complex.name, "alice"))
            .prepare();

        // Execute — only the placeholder needs to be bound
        let update_count = drizzle_exec!(prepared.execute(db.conn(), [new_age.bind(30)]));
        drizzle_assert_eq!(1, update_count, "Should have updated one row");

        // Verify both concrete and placeholder-bound fields were updated
        let results: Vec<ComplexResult> = drizzle_exec!(
            db.select((complex.name, complex.email, complex.age, complex.score))
                .from(complex)
                .r#where(eq(complex.name, "alice"))
                => all
        );

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].email,
            Some("alice@new.com".to_string()),
            "Concrete value should be updated"
        );
        assert_eq!(
            results[0].age,
            Some(30),
            "Placeholder-bound value should be updated"
        );
        assert_eq!(
            results[0].score,
            Some(90.5),
            "Untouched field should remain unchanged"
        );
    }
);

#[cfg(feature = "uuid")]
sqlite_test!(test_update_skip_excludes_unset_fields, ComplexSchema, {
    let ComplexSchema { complex } = schema;

    // Set only email — all other fields remain Skip (default)
    let update = UpdateComplex::default().with_email("only-this@test.com".to_string());

    let stmt = db
        .update(complex)
        .set(update)
        .r#where(eq(complex.name, "someone"));

    let sql_string = stmt.to_sql().sql();

    // SET clause should contain only the email column
    assert!(
        sql_string.contains("\"email\""),
        "SQL should include email in SET, got: {}",
        sql_string
    );

    // Other columns should NOT appear in SET (they're all Skip)
    assert!(
        !sql_string.contains("\"age\""),
        "SQL should NOT include age (it was Skip), got: {}",
        sql_string
    );
    assert!(
        !sql_string.contains("\"score\""),
        "SQL should NOT include score (it was Skip), got: {}",
        sql_string
    );
    assert!(
        !sql_string.contains("\"active\""),
        "SQL should NOT include active (it was Skip), got: {}",
        sql_string
    );
    assert!(
        !sql_string.contains("\"description\""),
        "SQL should NOT include description (it was Skip), got: {}",
        sql_string
    );
});
