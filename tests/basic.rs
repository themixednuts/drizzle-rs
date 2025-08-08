use cfg_if::cfg_if;
use common::{Complex, InsertComplex, InsertSimple, SelectSimple, Simple, setup_db};
use drizzle_core::sql;
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
    let (mut drizzle, (simple, complex)) = drizzle!(db, [Simple, Complex]);

    let data = InsertSimple::default().with_name("test");
    let inserted = drizzle.insert(simple).values([data]).execute().unwrap();

    assert_eq!(inserted, 1);

    let selected: Vec<SelectSimple> = drizzle.select(()).from(simple).all().unwrap();

    let row: PartialSimple = drizzle.select(simple.name).from(simple).get().unwrap();

    assert_eq!(row.name, "test");

    // Test the new FromRow derive macro
    let derived_row: DerivedPartialSimple = drizzle.select(simple.name).from(simple).get().unwrap();

    assert_eq!(derived_row.name, "test");
}

cfg_if!(
    if #[cfg(feature = "uuid")] {
        #[derive(Debug)]
        struct PartialComplex {
            id: Uuid,
            name: String,
        }
    } else {
        #[derive(Debug)]
        struct PartialComplex {
            id: String,
            name: String,
        }
    }
);

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
    let (drizzle, (simple, complex)) = drizzle!(db, [Simple, Complex]);

    drizzle
        .insert(simple)
        .values([InsertSimple::default().with_id(1).with_name("simple")])
        .execute()
        .unwrap();

    let complex_data = InsertComplex::default().with_name("complex");

    drizzle
        .insert(complex)
        .values([complex_data])
        .execute()
        .unwrap();

    let simple: SelectSimple = drizzle
        .select(columns![Simple::id, Simple::name])
        .from(simple)
        .get()
        .unwrap();

    let sql: Vec<SelectComplex> = drizzle.select(()).from(complex).all().unwrap();
    println!("{sql:?}");

    let complex: PartialComplex = drizzle
        .select(sql![[Complex::id, Complex::name]])
        .from(complex)
        .get()
        .unwrap();

    assert_eq!(simple.name, "simple");
    assert_eq!(complex.name, "complex");
}

#[test]
fn test_from_row_derive_with_simple_struct() {
    let db = setup_db();
    let (drizzle, (simple, ..)) = drizzle!(db, [Simple]);

    let data = InsertSimple::default().with_name("derive_test");
    drizzle.insert(simple).values([data]).execute().unwrap();

    // Test the derived implementation
    let result: DerivedPartialSimple = drizzle
        .select(columns![Simple::name])
        .from(simple)
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
    use drizzle_core::sql;
    use uuid::Uuid;
    let db = setup_db();
    let (mut drizzle, (complex, ..)) = drizzle!(db, [Complex]);

    let select_sql = drizzle.select(complex.id).from(complex).to_sql();
    println!("SelectSQL {select_sql:?}");
    let sql = select_sql.sql();
    println!("SelectSQL {sql}");

    let c = Complex::default();
    c.id;

    drop(select_sql);

    // Insert a UUID directly using rusqlite to see how it's stored
    let test_uuid = Uuid::new_v4();
    println!("Generated UUID: {}", test_uuid);
    println!("UUID as string: {}", test_uuid.to_string());
    println!("UUID as bytes: {:?}", test_uuid.as_bytes());

    // Insert using drizzle
    let complex_data = InsertComplex::default().with_name("debug_test");
    println!("InsertCoplex SQL: {:?}", complex_data.to_sql());

    drizzle
        .insert(complex)
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
