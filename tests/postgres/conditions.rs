//! PostgreSQL condition expression tests

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::*;
use drizzle::core::expr::*;
use drizzle::postgres::prelude::*;
use drizzle_macros::postgres_test;

#[allow(dead_code)]
#[derive(Debug, PostgresFromRow)]
struct PgSimpleResult {
    id: i32,
    name: String,
}

#[allow(dead_code)]
#[cfg(feature = "uuid")]
#[derive(Debug, PostgresFromRow)]
struct PgComplexResult {
    id: uuid::Uuid,
    name: String,
    email: Option<String>,
    age: Option<i32>,
    active: bool,
}

postgres_test!(condition_eq, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .insert(simple)
        .values([InsertSimple::new("Alice"), InsertSimple::new("Bob")]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, "Alice"));
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Alice");
});

postgres_test!(condition_neq, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .insert(simple)
        .values([InsertSimple::new("Alice"), InsertSimple::new("Bob")]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(neq(simple.name, "Alice"));
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Bob");
});

#[cfg(feature = "uuid")]
postgres_test!(condition_gt_lt, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    let stmt = db.insert(complex).values([
        InsertComplex::new("Young", true, Role::User).with_age(20),
        InsertComplex::new("Middle", true, Role::User).with_age(40),
        InsertComplex::new("Senior", true, Role::User).with_age(60),
    ]);
    drizzle_exec!(stmt.execute());

    // Test gt
    let stmt = db.select(()).from(complex).r#where(gt(complex.age, 30));
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());
    assert_eq!(results.len(), 2);

    // Test lt
    let stmt = db.select(()).from(complex).r#where(lt(complex.age, 50));
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());
    assert_eq!(results.len(), 2);

    // Test gte
    let stmt = db.select(()).from(complex).r#where(gte(complex.age, 40));
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());
    assert_eq!(results.len(), 2);

    // Test lte
    let stmt = db.select(()).from(complex).r#where(lte(complex.age, 40));
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());
    assert_eq!(results.len(), 2);
});

postgres_test!(condition_in_array, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([
        InsertSimple::new("Alice"),
        InsertSimple::new("Bob"),
        InsertSimple::new("Charlie"),
        InsertSimple::new("David"),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(in_array(simple.name, ["Alice", "Charlie"]));
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 2);
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"Alice"));
    assert!(names.contains(&"Charlie"));
});

postgres_test!(condition_not_in_array, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([
        InsertSimple::new("Alice"),
        InsertSimple::new("Bob"),
        InsertSimple::new("Charlie"),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(not_in_array(simple.name, ["Alice"]));
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 2);
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"Bob"));
    assert!(names.contains(&"Charlie"));
});

#[cfg(feature = "uuid")]
postgres_test!(condition_is_null, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    // Separate inserts due to type state differences
    let stmt = db.insert(complex).values([
        InsertComplex::new("With Email", true, Role::User).with_email("test@example.com")
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .insert(complex)
        .values([InsertComplex::new("No Email", true, Role::User)]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(complex).r#where(is_null(complex.email));
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "No Email");
});

#[cfg(feature = "uuid")]
postgres_test!(condition_is_not_null, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    // Separate inserts due to type state differences
    let stmt = db.insert(complex).values([
        InsertComplex::new("With Email", true, Role::User).with_email("test@example.com")
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .insert(complex)
        .values([InsertComplex::new("No Email", true, Role::User)]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select(())
        .from(complex)
        .r#where(is_not_null(complex.email));
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "With Email");
});

postgres_test!(condition_like, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([
        InsertSimple::new("test_one"),
        InsertSimple::new("test_two"),
        InsertSimple::new("other"),
    ]);
    drizzle_exec!(stmt.execute());

    // Prefix match
    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(like(simple.name, "test%"));
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());
    assert_eq!(results.len(), 2);

    // Contains match
    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(like(simple.name, "%o%"));
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());
    assert_eq!(results.len(), 3); // test_one, test_two, other all contain 'o'
});

#[cfg(feature = "uuid")]
postgres_test!(condition_between, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    let stmt = db.insert(complex).values([
        InsertComplex::new("Teen", true, Role::User).with_age(15),
        InsertComplex::new("Young", true, Role::User).with_age(25),
        InsertComplex::new("Adult", true, Role::User).with_age(45),
        InsertComplex::new("Senior", true, Role::User).with_age(75),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select(())
        .from(complex)
        .r#where(between(complex.age, 20, 50));
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 2);
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"Young"));
    assert!(names.contains(&"Adult"));
});

#[cfg(feature = "uuid")]
postgres_test!(condition_and, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    let stmt = db.insert(complex).values([
        InsertComplex::new("Active Young", true, Role::User).with_age(25),
        InsertComplex::new("Inactive Young", false, Role::User).with_age(25),
        InsertComplex::new("Active Old", true, Role::User).with_age(60),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select(())
        .from(complex)
        .r#where(and([eq(complex.active, true), lt(complex.age, 30)]));
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Active Young");
});

#[cfg(feature = "uuid")]
postgres_test!(condition_or, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    let stmt = db.insert(complex).values([
        InsertComplex::new("Admin", true, Role::Admin),
        InsertComplex::new("Moderator", true, Role::Moderator),
        InsertComplex::new("User", true, Role::User),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(complex).r#where(or([
        eq(complex.role, Role::Admin),
        eq(complex.role, Role::Moderator),
    ]));
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 2);
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"Admin"));
    assert!(names.contains(&"Moderator"));
});

#[cfg(feature = "uuid")]
postgres_test!(condition_nested_and_or, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    let stmt = db.insert(complex).values([
        InsertComplex::new("Active Admin", true, Role::Admin).with_age(30),
        InsertComplex::new("Inactive Admin", false, Role::Admin).with_age(30),
        InsertComplex::new("Active User Young", true, Role::User).with_age(20),
        InsertComplex::new("Active User Old", true, Role::User).with_age(50),
    ]);
    drizzle_exec!(stmt.execute());

    // (Admin OR Moderator) AND active AND age > 25
    let stmt = db.select(()).from(complex).r#where(and([
        or([
            eq(complex.role, Role::Admin),
            eq(complex.role, Role::Moderator),
        ]),
        eq(complex.active, true),
        gt(complex.age, 25),
    ]));
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Active Admin");
});

#[cfg(feature = "uuid")]
postgres_test!(condition_not, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    let stmt = db.insert(complex).values([
        InsertComplex::new("Active", true, Role::User),
        InsertComplex::new("Inactive", false, Role::User),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select(())
        .from(complex)
        .r#where(not(eq(complex.active, true)));
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Inactive");
});
