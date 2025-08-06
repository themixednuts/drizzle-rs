use common::{Complex, InsertComplex, InsertSimple, SelectSimple, Simple, setup_db};
use drizzle_rs::prelude::*;
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

#[derive(Debug)]
struct PartialComplex {
    id: Uuid,
    name: String,
}
impl TryFrom<&Row<'_>> for PartialComplex {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'_>) -> std::result::Result<PartialComplex, rusqlite::Error> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
        })
    }
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
}

#[test]
fn multiple_tables() {
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

    let complex: PartialComplex = drizzle
        .select(columns![Complex::id, Complex::name])
        .from::<Complex>()
        .get()
        .unwrap();

    assert_eq!(simple.name, "simple");
    assert_eq!(complex.name, "complex");
}
