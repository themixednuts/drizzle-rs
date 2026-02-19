use drizzle::core::expr::CastTarget;
use drizzle::core::types as core_types;
use drizzle::postgres::types as postgres_types;

#[test]
fn postgres_dialect_types_are_distinct_markers_with_cast_mappings() {
    fn assert_target<T: core_types::DataType, C: CastTarget<'static, T>>(_: C) {}

    let _ = postgres_types::Int2;
    let _ = postgres_types::Int4;
    let _ = postgres_types::Int8;
    let _ = postgres_types::Float4;
    let _ = postgres_types::Float8;
    let _ = postgres_types::Varchar;
    let _ = postgres_types::Bytea;
    let _ = postgres_types::Boolean;
    let _ = postgres_types::Timestamptz;

    assert_target::<core_types::SmallInt, _>(postgres_types::Int2);
    assert_target::<core_types::Int, _>(postgres_types::Int4);
    assert_target::<core_types::BigInt, _>(postgres_types::Int8);
    assert_target::<core_types::Float, _>(postgres_types::Float4);
    assert_target::<core_types::Double, _>(postgres_types::Float8);
    assert_target::<core_types::VarChar, _>(postgres_types::Varchar);
    assert_target::<core_types::Bytes, _>(postgres_types::Bytea);
    assert_target::<core_types::Bool, _>(postgres_types::Boolean);
    assert_target::<core_types::TimestampTz, _>(postgres_types::Timestamptz);
}
