#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
use std::marker::PhantomData;

use common::{Complex, InsertComplex, InsertSimple, Role, SelectComplex, SelectSimple, Simple};
use drizzle_rs::prelude::*;

#[cfg(feature = "rusqlite")]
use crate::common::PartialSelectSimple;

mod common;

// Test the new FromRow derive macro
#[derive(FromRow, Debug, Default)]
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

#[tokio::test]
async fn basic_insert_select() {
    let conn = setup_test_db!();
    let (db, simple) = drizzle!(conn, [Simple]);

    let data = InsertSimple::default().with_name("test");
    let inserted = drizzle_exec!(db.insert(simple).values([data]).execute());
    assert_eq!(inserted, 1);

    let selected: Vec<SelectSimple> = drizzle_exec!(db.select(()).from(simple).all());
    assert!(selected.len() > 0);
    assert_eq!(selected[0].name, "test");

    #[cfg(feature = "rusqlite")]
    {
        let row: PartialSelectSimple = drizzle_exec!(db.select(simple.name).from(simple).get());
        assert_eq!(row.name, Some("test".into()));
    }

    // Test the new FromRow derive macro
    let derived_row: DerivedPartialSimple =
        drizzle_exec!(db.select(simple.name).from(simple).get());
    assert_eq!(derived_row.name, "test");
}

#[cfg(feature = "uuid")]
#[tokio::test]
async fn multiple_tables() {
    let conn = setup_test_db!();
    let (drizzle, (simple, complex)) = drizzle!(conn, [Simple, Complex]);

    let inserted = drizzle_exec!(
        drizzle
            .insert(simple)
            .values([InsertSimple::default().with_id(1).with_name("simple")])
            .execute()
    );

    let complex_data = InsertComplex::default()
        .with_name("complex")
        .with_active(true)
        .with_role(Role::User);

    let inserted = drizzle_exec!(drizzle.insert(complex).values([complex_data]).execute());

    let simple: SelectSimple =
        drizzle_exec!(drizzle.select((simple.id, simple.name)).from(simple).get());

    let sql: Vec<SelectComplex> = drizzle_exec!(drizzle.select(()).from(complex).all());
    println!("{sql:?}");

    #[cfg(feature = "rusqlite")]
    {
        use crate::common::PartialSelectComplex;
        let complex: PartialSelectComplex = drizzle_exec!(
            drizzle
                .select((complex.id, complex.name))
                .from(complex)
                .get()
        );
        assert_eq!(complex.name, Some("complex".into()));
    }

    #[cfg(any(feature = "turso", feature = "libsql"))]
    {
        let complex: SelectComplex = drizzle_exec!(drizzle.select(()).from(complex).get());
        assert_eq!(complex.name, "complex");
    }

    assert_eq!(simple.name, "simple");
}

#[tokio::test]
async fn test_from_row_derive_with_simple_struct() {
    let conn = setup_test_db!();
    let (drizzle, simple) = drizzle!(conn, [Simple]);

    let data = InsertSimple::new("derive_test");
    drizzle_exec!(drizzle.insert(simple).values([data]).execute());

    // Test the derived implementation
    let result: DerivedPartialSimple =
        drizzle_exec!(drizzle.select(simple.name).from(simple).get());
    assert_eq!(result.name, "derive_test");
}

#[tokio::test]
async fn test_from_row_with_column_mapping() {
    let conn = setup_test_db!();
    let (drizzle, simple) = drizzle!(conn, [Simple]);

    let data = InsertSimple::new("column_test").with_id(42);
    drizzle_exec!(drizzle.insert(simple).values([data]).execute());

    // Test the column-mapped FromRow implementation
    let result: DerivedSimpleWithColumns = drizzle_exec!(
        drizzle
            .select(DerivedSimpleWithColumns::default())
            .from(simple)
            .get()
    );

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
#[tokio::test]
async fn debug_uuid_storage() {
    use crate::common::Role;
    use uuid::Uuid;

    let conn = setup_test_db!();
    let (drizzle, complex) = drizzle!(conn, [Complex]);

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

    drizzle_exec!(drizzle.insert(complex).values([complex_data]).execute());

    // Check what's actually in the database using raw SQL
    let query = "SELECT typeof(id) as id_type FROM complex LIMIT 1";
    let mut stmt = prepare_stmt!(drizzle.conn(), query);

    #[cfg(feature = "rusqlite")]
    {
        let id_type: String = stmt
            .query_row([], |row| Ok(row.get::<_, String>(0)?))
            .unwrap();
        println!("Database ID type: {}", id_type);
    }

    #[cfg(any(feature = "turso", feature = "libsql"))]
    {
        query_row!(stmt, db_params!(), |row| {
            let id_type = row_get!(row, 0, String);
            println!("Database ID type: {}", id_type);
        });
    }
}
