use common::{Complex, InsertComplex, InsertSimple, Simple, UpdateComplex, UpdateSimple};
use drizzle_rs::prelude::*;
#[cfg(feature = "rusqlite")]
use rusqlite::Row;
#[cfg(feature = "uuid")]
use uuid::Uuid;

mod common;

#[derive(FromRow, Debug)]
struct SimpleResult {
    id: i32,
    name: String,
}

#[cfg(not(feature = "uuid"))]
#[derive(Debug)]
struct ComplexResult {
    id: i32,
    name: String,
    email: Option<String>,
    age: Option<i32>,
    description: Option<String>,
}

#[cfg(feature = "uuid")]
#[derive(FromRow, Debug)]
struct ComplexResult {
    id: Uuid, // UUID stored as string
    name: String,
    email: Option<String>,
    age: Option<i32>,
    description: Option<String>,
}

#[tokio::test]
async fn end_to_end_workflow() {
    let db = setup_test_db!();
    let (drizzle, (simple, complex)) = drizzle!(db, [Simple, Complex]);

    // Insert Simple record
    let simple_insert = InsertSimple::new("test_simple");
    let simple_rows = drizzle_exec!(drizzle.insert(simple).values([simple_insert]).execute());
    assert_eq!(simple_rows, 1);

    // Insert Complex record
    #[cfg(not(feature = "uuid"))]
    let complex_insert = InsertComplex::new("test_complex", true, common::Role::User)
        .with_email("test@example.com".to_string())
        .with_age(25)
        .with_description("A test record".to_string());

    #[cfg(feature = "uuid")]
    let complex_insert = InsertComplex::new("test_complex", true, common::Role::User)
        .with_id(uuid::Uuid::new_v4())
        .with_email("test@example.com".to_string())
        .with_age(25)
        .with_description("A test record".to_string());

    let complex_rows = drizzle_exec!(drizzle.insert(complex).values([complex_insert]).execute());
    assert_eq!(complex_rows, 1);

    // Verify Simple record was inserted
    let simple_results: Vec<SimpleResult> = drizzle_exec!(
        drizzle
            .select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "test_simple"))
            .all()
    );

    assert_eq!(simple_results.len(), 1);
    assert_eq!(simple_results[0].name, "test_simple");

    // Verify Complex record was inserted
    let complex_results: Vec<ComplexResult> = drizzle_exec!(
        drizzle
            .select((
                complex.id,
                complex.name,
                complex.email,
                complex.age,
                complex.description,
            ))
            .from(complex)
            .r#where(eq(complex.name, "test_complex"))
            .all()
    );

    assert_eq!(complex_results.len(), 1);
    assert_eq!(complex_results[0].name, "test_complex");
    assert_eq!(
        complex_results[0].email,
        Some("test@example.com".to_string())
    );
    assert_eq!(complex_results[0].age, Some(25));
    assert_eq!(
        complex_results[0].description,
        Some("A test record".to_string())
    );

    // TODO fix Update models, they need a new function for required, and use Insert's null/omit handling
    // Update Simple record
    let simple_update_rows = drizzle_exec!(
        drizzle
            .update(simple)
            .set(UpdateSimple::default().with_name("updated_simple"))
            .r#where(eq(Simple::name, "test_simple"))
            .execute()
    );
    assert_eq!(simple_update_rows, 1);

    // Update Complex record
    let complex_update_rows = drizzle_exec!(
        drizzle
            .update(complex)
            .set(
                UpdateComplex::default()
                    .with_email("updated@example.com".to_string())
                    .with_age(30)
                    .with_description("Updated description".to_string()),
            )
            .r#where(eq(Complex::name, "test_complex"))
            .execute()
    );
    assert_eq!(complex_update_rows, 1);

    // Verify Simple record was updated
    let updated_simple_results: Vec<SimpleResult> = drizzle_exec!(
        drizzle
            .select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "updated_simple"))
            .all()
    );

    assert_eq!(updated_simple_results.len(), 1);
    assert_eq!(updated_simple_results[0].name, "updated_simple");

    // Verify old Simple record name is gone
    let old_simple_results: Vec<SimpleResult> = drizzle_exec!(
        drizzle
            .select((simple.id, simple.name))
            .from(simple)
            .r#where(eq(simple.name, "test_simple"))
            .all()
    );

    assert_eq!(old_simple_results.len(), 0);

    // Verify Complex record was updated
    let updated_complex_results: Vec<ComplexResult> = drizzle_exec!(
        drizzle
            .select((
                complex.id,
                complex.name,
                complex.email,
                complex.age,
                complex.description,
            ))
            .from(complex)
            .r#where(eq(complex.name, "test_complex"))
            .all()
    );

    assert_eq!(updated_complex_results.len(), 1);
    assert_eq!(
        updated_complex_results[0].email,
        Some("updated@example.com".to_string())
    );
    assert_eq!(updated_complex_results[0].age, Some(30));
    assert_eq!(
        updated_complex_results[0].description,
        Some("Updated description".to_string())
    );

    // Delete Simple record
    let simple_delete_rows = drizzle_exec!(
        drizzle
            .delete(simple)
            .r#where(eq(simple.name, "updated_simple"))
            .execute()
    );
    assert_eq!(simple_delete_rows, 1);

    // Delete Complex record
    let complex_delete_rows = drizzle_exec!(
        drizzle
            .delete(complex)
            .r#where(eq(complex.name, "test_complex"))
            .execute()
    );
    assert_eq!(complex_delete_rows, 1);

    // Verify Simple record was deleted
    let deleted_simple_results: Vec<SimpleResult> =
        drizzle_exec!(drizzle.select((simple.id, simple.name)).from(simple).all());

    assert_eq!(deleted_simple_results.len(), 0);

    // Verify Complex record was deleted
    let deleted_complex_results: Vec<ComplexResult> = drizzle_exec!(
        drizzle
            .select((
                complex.id,
                complex.name,
                complex.email,
                complex.age,
                complex.description,
            ))
            .from(complex)
            .all()
    );

    assert_eq!(deleted_complex_results.len(), 0);
}
