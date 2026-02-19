use drizzle::core::types as core_types;
use drizzle::postgres::types as postgres_types;

#[test]
fn postgres_type_aliases_map_to_core_types() {
    let _: postgres_types::Int2 = core_types::SmallInt;
    let _: postgres_types::Int4 = core_types::Int;
    let _: postgres_types::Int8 = core_types::BigInt;
    let _: postgres_types::Float4 = core_types::Float;
    let _: postgres_types::Float8 = core_types::Double;
    let _: postgres_types::Varchar = core_types::VarChar;
    let _: postgres_types::Bytea = core_types::Bytes;
    let _: postgres_types::Boolean = core_types::Bool;
    let _: postgres_types::Timestamptz = core_types::TimestampTz;
}
