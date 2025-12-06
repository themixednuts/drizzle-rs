//! PostgreSQL condition expression tests

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::pg::*;
use drizzle::postgres::prelude::*;
use drizzle_macros::postgres_test;

#[derive(Debug, PostgresFromRow)]
struct PgSimpleResult {
    id: i32,
    name: String,
}

#[cfg(feature = "uuid")]
#[derive(Debug, PostgresFromRow)]
struct PgComplexResult {
    id: uuid::Uuid,
    name: String,
    email: Option<String>,
    age: Option<i32>,
    active: bool,
}

postgres_test!(condition_eq, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    let stmt = db
        .insert(simple)
        .values([InsertPgSimple::new("Alice"), InsertPgSimple::new("Bob")]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, "Alice"));
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Alice");
});

postgres_test!(condition_neq, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    let stmt = db
        .insert(simple)
        .values([InsertPgSimple::new("Alice"), InsertPgSimple::new("Bob")]);
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
postgres_test!(condition_gt_lt, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    let stmt = db.insert(complex).values([
        InsertPgComplex::new("Young", true, PgRole::User).with_age(20),
        InsertPgComplex::new("Middle", true, PgRole::User).with_age(40),
        InsertPgComplex::new("Senior", true, PgRole::User).with_age(60),
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

postgres_test!(condition_in_array, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([
        InsertPgSimple::new("Alice"),
        InsertPgSimple::new("Bob"),
        InsertPgSimple::new("Charlie"),
        InsertPgSimple::new("David"),
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

postgres_test!(condition_not_in_array, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([
        InsertPgSimple::new("Alice"),
        InsertPgSimple::new("Bob"),
        InsertPgSimple::new("Charlie"),
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
postgres_test!(condition_is_null, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    // Separate inserts due to type state differences
    let stmt =
        db.insert(complex)
            .values([InsertPgComplex::new("With Email", true, PgRole::User)
                .with_email("test@example.com")]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .insert(complex)
        .values([InsertPgComplex::new("No Email", true, PgRole::User)]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(complex).r#where(is_null(complex.email));
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "No Email");
});

#[cfg(feature = "uuid")]
postgres_test!(condition_is_not_null, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    // Separate inserts due to type state differences
    let stmt =
        db.insert(complex)
            .values([InsertPgComplex::new("With Email", true, PgRole::User)
                .with_email("test@example.com")]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .insert(complex)
        .values([InsertPgComplex::new("No Email", true, PgRole::User)]);
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select(())
        .from(complex)
        .r#where(is_not_null(complex.email));
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "With Email");
});

postgres_test!(condition_like, PgSimpleSchema, {
    let PgSimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([
        InsertPgSimple::new("test_one"),
        InsertPgSimple::new("test_two"),
        InsertPgSimple::new("other"),
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
postgres_test!(condition_between, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    let stmt = db.insert(complex).values([
        InsertPgComplex::new("Teen", true, PgRole::User).with_age(15),
        InsertPgComplex::new("Young", true, PgRole::User).with_age(25),
        InsertPgComplex::new("Adult", true, PgRole::User).with_age(45),
        InsertPgComplex::new("Senior", true, PgRole::User).with_age(75),
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
postgres_test!(condition_and, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    let stmt = db.insert(complex).values([
        InsertPgComplex::new("Active Young", true, PgRole::User).with_age(25),
        InsertPgComplex::new("Inactive Young", false, PgRole::User).with_age(25),
        InsertPgComplex::new("Active Old", true, PgRole::User).with_age(60),
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
postgres_test!(condition_or, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    let stmt = db.insert(complex).values([
        InsertPgComplex::new("Admin", true, PgRole::Admin),
        InsertPgComplex::new("Moderator", true, PgRole::Moderator),
        InsertPgComplex::new("User", true, PgRole::User),
    ]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(complex).r#where(or([
        eq(complex.role, PgRole::Admin),
        eq(complex.role, PgRole::Moderator),
    ]));
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 2);
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"Admin"));
    assert!(names.contains(&"Moderator"));
});

#[cfg(feature = "uuid")]
postgres_test!(condition_nested_and_or, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    let stmt = db.insert(complex).values([
        InsertPgComplex::new("Active Admin", true, PgRole::Admin).with_age(30),
        InsertPgComplex::new("Inactive Admin", false, PgRole::Admin).with_age(30),
        InsertPgComplex::new("Active User Young", true, PgRole::User).with_age(20),
        InsertPgComplex::new("Active User Old", true, PgRole::User).with_age(50),
    ]);
    drizzle_exec!(stmt.execute());

    // (Admin OR Moderator) AND active AND age > 25
    let stmt = db.select(()).from(complex).r#where(and([
        or([
            eq(complex.role, PgRole::Admin),
            eq(complex.role, PgRole::Moderator),
        ]),
        eq(complex.active, true),
        gt(complex.age, 25),
    ]));
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Active Admin");
});

#[cfg(feature = "uuid")]
postgres_test!(condition_not, PgComplexSchema, {
    let PgComplexSchema { complex, .. } = schema;

    let stmt = db.insert(complex).values([
        InsertPgComplex::new("Active", true, PgRole::User),
        InsertPgComplex::new("Inactive", false, PgRole::User),
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
