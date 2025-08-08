use common::{Complex, InsertComplex, InsertSimple, Simple, UpdateComplex, UpdateSimple, setup_db};
use drizzle_rs::prelude::*;
use rusqlite::Row;

mod common;

#[derive(Debug)]
struct SimpleResult {
    id: i32,
    name: String,
}

impl TryFrom<&Row<'_>> for SimpleResult {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> std::result::Result<SimpleResult, rusqlite::Error> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    }
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
#[derive(Debug)]
struct ComplexResult {
    id: Uuid, // UUID stored as string
    name: String,
    email: Option<String>,
    age: Option<i32>,
    description: Option<String>,
}

impl TryFrom<&Row<'_>> for ComplexResult {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> std::result::Result<ComplexResult, rusqlite::Error> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            email: row.get(2)?,
            age: row.get(3)?,
            description: row.get(4)?,
        })
    }
}

#[test]
fn end_to_end_workflow() {
    let db = setup_db();
    let (drizzle, (simple, complex)) = drizzle!(db, [Simple, Complex]);

    // === PHASE 1: INSERT ===

    // Insert Simple record
    let simple_insert = InsertSimple::default().with_name("test_simple");
    let simple_rows = drizzle
        .insert(simple)
        .values([simple_insert])
        .execute()
        .unwrap();
    assert_eq!(simple_rows, 1);

    // Insert Complex record
    #[cfg(not(feature = "uuid"))]
    let complex_insert = InsertComplex::default()
        .with_name("test_complex")
        .with_email("test@example.com".to_string())
        .with_age(25)
        .with_description("A test record".to_string());

    #[cfg(feature = "uuid")]
    let complex_insert = InsertComplex::default()
        .with_id(uuid::Uuid::new_v4())
        .with_name("test_complex")
        .with_email("test@example.com".to_string())
        .with_age(25)
        .with_description("A test record".to_string());

    let complex_rows = drizzle
        .insert(complex)
        .values([complex_insert])
        .execute()
        .unwrap();
    assert_eq!(complex_rows, 1);

    // === PHASE 2: VERIFY INSERTION ===

    // Verify Simple record was inserted
    let simple_results: Vec<SimpleResult> = drizzle
        .select(columns![Simple::id, Simple::name])
        .from(simple)
        .r#where(eq(Simple::name, "test_simple"))
        .all()
        .unwrap();

    assert_eq!(simple_results.len(), 1);
    assert_eq!(simple_results[0].name, "test_simple");

    // Verify Complex record was inserted
    let complex_results: Vec<ComplexResult> = drizzle
        .select(columns![
            Complex::id,
            Complex::name,
            Complex::email,
            Complex::age,
            Complex::description,
        ])
        .from(complex)
        .r#where(eq(Complex::name, "test_complex"))
        .all()
        .unwrap();

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

    // === PHASE 3: UPDATE ===

    // Update Simple record
    let simple_update_rows = drizzle
        .update(simple)
        .set(UpdateSimple::default().with_name("updated_simple"))
        .r#where(eq(Simple::name, "test_simple"))
        .execute()
        .unwrap();
    assert_eq!(simple_update_rows, 1);

    // Update Complex record
    let complex_update_rows = drizzle
        .update(complex)
        .set(
            UpdateComplex::default()
                .with_email("updated@example.com".to_string())
                .with_age(30)
                .with_description("Updated description".to_string()),
        )
        .r#where(eq(Complex::name, "test_complex"))
        .execute()
        .unwrap();
    assert_eq!(complex_update_rows, 1);

    // === PHASE 4: VERIFY UPDATES ===

    // Verify Simple record was updated
    let updated_simple_results: Vec<SimpleResult> = drizzle
        .select(columns![Simple::id, Simple::name])
        .from(simple)
        .r#where(eq(Simple::name, "updated_simple"))
        .all()
        .unwrap();

    assert_eq!(updated_simple_results.len(), 1);
    assert_eq!(updated_simple_results[0].name, "updated_simple");

    // Verify old Simple record name is gone
    let old_simple_results: Vec<SimpleResult> = drizzle
        .select(columns![Simple::id, Simple::name])
        .from(simple)
        .r#where(eq(Simple::name, "test_simple"))
        .all()
        .unwrap();

    assert_eq!(old_simple_results.len(), 0);

    // Verify Complex record was updated
    let updated_complex_results: Vec<ComplexResult> = drizzle
        .select(columns![
            Complex::id,
            Complex::name,
            Complex::email,
            Complex::age,
            Complex::description,
        ])
        .from(complex)
        .r#where(eq(Complex::name, "test_complex"))
        .all()
        .unwrap();

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

    // === PHASE 5: DELETE ===

    // Delete Simple record
    let simple_delete_rows = drizzle
        .delete(simple)
        .r#where(eq(Simple::name, "updated_simple"))
        .execute()
        .unwrap();
    assert_eq!(simple_delete_rows, 1);

    // Delete Complex record
    let complex_delete_rows = drizzle
        .delete(complex)
        .r#where(eq(Complex::name, "test_complex"))
        .execute()
        .unwrap();
    assert_eq!(complex_delete_rows, 1);

    // === PHASE 6: VERIFY DELETION ===

    // Verify Simple record was deleted
    let deleted_simple_results: Vec<SimpleResult> = drizzle
        .select(columns![Simple::id, Simple::name])
        .from(simple)
        .all()
        .unwrap();

    assert_eq!(deleted_simple_results.len(), 0);

    // Verify Complex record was deleted
    let deleted_complex_results: Vec<ComplexResult> = drizzle
        .select(columns![
            Complex::id,
            Complex::name,
            Complex::email,
            Complex::age,
            Complex::description,
        ])
        .from(complex)
        .all()
        .unwrap();

    assert_eq!(deleted_complex_results.len(), 0);
}
