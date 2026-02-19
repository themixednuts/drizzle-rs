use drizzle::core::types as core_types;
use drizzle::sqlite::types as sqlite_types;

#[test]
fn sqlite_type_aliases_map_to_core_types() {
    let _: sqlite_types::Integer = core_types::BigInt;
    let _: sqlite_types::Real = core_types::Double;
    let _: sqlite_types::Blob = core_types::Bytes;
}
