use common::{Complex, InsertComplex, InsertSimple, SelectSimple, Simple, setup_db};
use drizzle_core::sql;
use drizzle_rs::prelude::*;
use procmacros::FromRow;

use crate::common::PartialSelectSimple;

mod common;

// Test the new FromRow derive macro
#[derive(FromRow, Debug)]
struct DerivedPartialSimple {
    name: String,
}

// Test FromRow with column mapping
#[derive(FromRow, Debug, Default)]
struct DerivedSimpleWithColumns {
    #[column(Simple::id)]
    table_id: i32,
    #[column(Simple::name)]
    table_name: String,
}

#[test]
fn basic_insert_select() {
    let db = setup_db();
    let (db, simple) = drizzle!(db, [Simple]);

    let data = InsertSimple::default().with_name("test");
    let inserted = db.insert(simple).values([data]).execute().unwrap();

    assert_eq!(inserted, 1);

    let selected: Vec<SelectSimple> = db.select(()).from(simple).all().unwrap();

    assert!(selected.len() > 0);
    assert_eq!(selected[0].name, "test");

    let row: PartialSelectSimple = db.select(simple.name).from(simple).get().unwrap();

    assert_eq!(row.name, Some("test".into()));

    // Test the new FromRow derive macro
    let derived_row: DerivedPartialSimple = db.select(simple.name).from(simple).get().unwrap();

    assert_eq!(derived_row.name, "test");
}

#[cfg(feature = "uuid")]
#[test]
fn multiple_tables() {
    use common::SelectComplex;

    use crate::common::{PartialSelectComplex, Role};

    let db = setup_db();
    let (drizzle, (simple, complex)) = drizzle!(db, [Simple, Complex]);

    drizzle
        .insert(simple)
        .values([InsertSimple::default().with_id(1).with_name("simple")])
        .execute()
        .unwrap();

    let complex_data = InsertComplex::default()
        .with_name("complex")
        .with_active(true)
        .with_role(Role::User);

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

    let complex: PartialSelectComplex = drizzle
        .select(sql![[complex.id, complex.name]])
        .from(complex)
        .get()
        .unwrap();

    assert_eq!(simple.name, "simple");
    assert_eq!(complex.name, Some("complex".into()));
}

#[test]
fn test_from_row_derive_with_simple_struct() {
    let db = setup_db();
    let (drizzle, simple) = drizzle!(db, [Simple]);

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

#[test]
fn test_from_row_with_column_mapping() {
    let db = setup_db();
    let (drizzle, simple) = drizzle!(db, [Simple]);

    let data = InsertSimple::default().with_id(42).with_name("column_test");
    drizzle.insert(simple).values([data]).execute().unwrap();

    // Test the column-mapped FromRow implementation
    let result: DerivedSimpleWithColumns = drizzle
        .select(DerivedSimpleWithColumns::default())
        .from(simple)
        .get()
        .unwrap();

    assert_eq!(result.table_id, 42);
    assert_eq!(result.table_name, "column_test");
}

#[cfg(feature = "uuid")]
#[test]
fn debug_schema() {
    println!(
        "Complex SQL schema: {}",
        Complex::SQL.to_sql().sql().as_str()
    );
}

#[cfg(feature = "uuid")]
#[test]
fn debug_uuid_storage() {
    use uuid::Uuid;

    use crate::common::Role;
    let db = setup_db();
    let (drizzle, complex) = drizzle!(db, [Complex]);

    let select_sql = drizzle.select(complex.id).from(complex).to_sql();
    println!("SelectSQL {select_sql:?}");
    let sql = select_sql.sql();
    println!("SelectSQL {sql}");

    // Insert a UUID directly using rusqlite to see how it's stored
    let test_uuid = Uuid::new_v4();
    println!("Generated UUID: {}", test_uuid);
    println!("UUID as string: {}", test_uuid.to_string());
    println!("UUID as bytes: {:?}", test_uuid.as_bytes());

    // Insert using drizzle
    let complex_data = InsertComplex::default()
        .with_name("debug_test")
        .with_active(false)
        .with_role(Role::User);
    println!("InsertCoplex SQL: {:?}", complex_data.to_sql());

    drizzle
        .insert(complex)
        .values([complex_data])
        .execute()
        .unwrap();

    // Check what's actually in the database using raw SQL
    let query = "SELECT typeof(id) as id_type FROM complex LIMIT 1";
    let mut stmt = drizzle.conn().prepare(query).unwrap();
    let id_type: String = stmt
        .query_row([], |row| Ok(row.get::<_, String>(0)?))
        .unwrap();

    println!("Database ID type: {}", id_type);
}
