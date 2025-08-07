use common::{Complex, InsertComplex, InsertSimple, SelectSimple, Simple, setup_db};
use drizzle_rs::prelude::*;
use procmacros::FromRow;
use rusqlite::{Row, Rows};

mod common;

#[derive(Debug)]
struct PartialSimple {
    name: String,
}

impl TryFrom<&Row<'_>> for PartialSimple {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> std::result::Result<PartialSimple, rusqlite::Error> {
        Ok(Self {
            name: row.get("name")?,
        })
    }
}

// Test the new FromRow derive macro
#[derive(FromRow, Debug)]
struct DerivedPartialSimple {
    name: String,
}

// Test FromRow with multiple fields and types
#[derive(FromRow, Debug)]
struct DerivedComplexResult {
    id: i32,
    name: String,
    email: Option<String>,
}

#[test]
fn basic_insert_select() {
    let db = setup_db();
    let mut drizzle = drizzle!(db, [Simple, Complex]);

    let data = InsertSimple::default().with_name("test");
    let inserted = drizzle.insert::<Simple>().values([data]).execute().unwrap();

    assert_eq!(inserted, 1);

    let selected: Vec<SelectSimple> = drizzle.select([()]).from::<Simple>().all().unwrap();

    let row: PartialSimple = drizzle
        .select(columns![Simple::name])
        .from::<Simple>()
        .get()
        .unwrap();

    assert_eq!(row.name, "test");

    // Test the new FromRow derive macro
    let derived_row: DerivedPartialSimple = drizzle
        .select(columns![Simple::name])
        .from::<Simple>()
        .get()
        .unwrap();

    assert_eq!(derived_row.name, "test");
}

#[cfg(feature = "uuid")]
#[derive(Debug)]
struct PartialComplex {
    id: Uuid,
    name: String,
}

#[cfg(feature = "uuid")]
impl TryFrom<&Row<'_>> for PartialComplex {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> std::result::Result<PartialComplex, rusqlite::Error> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
        })
    }
}

#[cfg(feature = "uuid")]
#[test]
fn multiple_tables() {
    use common::SelectComplex;

    let db = setup_db();
    let drizzle = drizzle!(db, [Simple, Complex]);

    drizzle
        .insert::<Simple>()
        .values([InsertSimple::default().with_id(1).with_name("simple")])
        .execute()
        .unwrap();

    let complex_data = InsertComplex::default().with_name("complex");

    drizzle
        .insert::<Complex>()
        .values([complex_data])
        .execute()
        .unwrap();

    let simple: SelectSimple = drizzle
        .select(columns![Simple::id, Simple::name])
        .from::<Simple>()
        .get()
        .unwrap();

    let sql: Vec<SelectComplex> = drizzle.select(()).from::<Complex>().all().unwrap();
    println!("{sql:?}");

    let complex: PartialComplex = drizzle
        .select(columns![Complex::id, Complex::name])
        .from::<Complex>()
        .get()
        .unwrap();

    assert_eq!(simple.name, "simple");
    assert_eq!(complex.name, "complex");
}

#[test]
fn test_from_row_derive_with_simple_struct() {
    let db = setup_db();
    let drizzle = drizzle!(db, [Simple]);

    let data = InsertSimple::default().with_name("derive_test");
    drizzle.insert::<Simple>().values([data]).execute().unwrap();

    // Test the derived implementation
    let result: DerivedPartialSimple = drizzle
        .select(columns![Simple::name])
        .from::<Simple>()
        .get()
        .unwrap();

    assert_eq!(result.name, "derive_test");
}

#[cfg(feature = "uuid")]
#[test]
fn debug_schema() {
    println!("Complex SQL schema: {}", Complex::SQL);
}

#[cfg(feature = "uuid")]
#[test]
fn debug_uuid_storage() {
    use uuid::Uuid;
    let db = setup_db();
    let mut drizzle = drizzle!(db, [Complex]);

    // Insert a UUID directly using rusqlite to see how it's stored
    let test_uuid = Uuid::new_v4();
    println!("Generated UUID: {}", test_uuid);
    println!("UUID as string: {}", test_uuid.to_string());
    println!("UUID as bytes: {:?}", test_uuid.as_bytes());

    // Insert using drizzle
    let complex_data = InsertComplex::default().with_name("debug_test");
    println!("InsertCoplex SQL: {:?}", complex_data.to_sql());

    drizzle
        .insert::<Complex>()
        .values([complex_data])
        .execute()
        .unwrap();

    // Check what's actually in the database using raw SQL
    let query = "SELECT typeof(id) as id_type FROM complex LIMIT 1";
    let mut stmt = drizzle.connection().prepare(query).unwrap();
    let id_type: String = stmt
        .query_row([], |row| Ok(row.get::<_, String>(0)?))
        .unwrap();

    println!("Database ID type: {}", id_type);
}
